extern crate lodepng;

use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::ffi::OsString;
use std::default::Default;

use lodepng::RGBA;

struct Config<'a>
{
    tile_size: usize,
    output_suffix: &'a str,
    make_backup: bool,
    input_paths: Vec<PathBuf>
}

#[derive(Debug, PartialEq)]
enum ArgsKey
{
    Default,
    TileSize,
    OutputSuffix
}

fn parse_args<'a>(args: &'a Vec<String>) -> Result<Config<'a>, Box<Error>>
{
    let mut tile_size: Option<usize> = None;
    let mut output_suffix = "";
    let mut input_paths = Vec::<PathBuf>::new();
    let mut make_backup = false;

    use ArgsKey::*;
    let mut current_key = Default;
    for arg in args.as_slice()
    {
        match current_key
        {
            TileSize =>
            {
                tile_size = Some(arg.parse().map_err(
                    |e| format!("{} (after --tile--size)", e)
                )?);
                current_key = Default;
            },

            OutputSuffix =>
            {
                output_suffix = &arg;
                current_key = Default;
            },

            Default =>
            {
                if arg.starts_with("--")
                {
                    current_key = match &arg.chars().as_str()[2..]
                    {
                        "tile-size" => TileSize,
                        "output-suffix" => OutputSuffix,
                        s => {
                            println!("Warning: Argument key {} is unknown (ignoring)", s);
                            Default
                        }
                    };
                }
                else
                {
                    input_paths.push(PathBuf::from(&arg));
                }
            }
        }
    }

    if current_key != Default
    {
        return Err(format!("Expected another argument (type {:?})", current_key).into());
    }

    if input_paths.len() == 0
    {
        return Err("No file paths specified".into());
    }

    if output_suffix == ""
    {
        make_backup = true;
        println!("Warning: No output suffix specified. Input file will be overwritten, and a backup (with suffix _backup) will be made. (use --output-suffix if you meant to specify a suffix)");
    }

    let c = Config
    {
        tile_size: tile_size.ok_or("No tile size specified (use --tile-size)")?,
        output_suffix,
        make_backup,
        input_paths
    };
    return Ok(c);
}

fn get_pixel_index(pixel_x: usize, pixel_y: usize, image_width: usize) -> usize
{
    pixel_y * image_width + pixel_x
}

fn insert_pixels(buf: &mut Vec<RGBA>, pos: usize, mut pixels: Vec<RGBA>) -> usize
{
    let count = pixels.len();

    if pos >= buf.len()
    {
        buf.append(&mut pixels);
    }
    else
    {
        let removed = buf.splice(pos..pos, pixels);
        assert!(removed.count() == 0);
    }

    return count;
}

fn process_image(config: &Config, path_i: usize) -> Result<(), Box<Error>>
{
    let input_path = &config.input_paths[path_i];

    println!("File: {:?}:", input_path);
    println!("  Processing with tile size {}", config.tile_size);

    //
    // Read file
    //

    let mut image = lodepng::decode32_file(input_path)?;

    println!("  Read {} pixels from file", image.buffer.len());

    //
    // Make backup if necessary
    //

    if config.make_backup
    {
        let mut backup_path = input_path.clone();
        let mut backup_name = backup_path.file_stem().ok_or("Invalid path")?.to_os_string();
        backup_name.push("_backup");
        backup_name.push(".png");
        backup_path.set_file_name(backup_name);

        lodepng::encode32_file(&backup_path, &image.buffer, image.width, image.height)?;

        println!("  Wrote {} pixels to {:?}", image.buffer.len(), OsString::from(backup_path));
    }

    //
    // Resize image
    //

    let columns = image.width / config.tile_size;
    let rows = image.height / config.tile_size;

    let new_width = image.width + columns * 2;
    let new_height = image.height + rows * 2;

    println!("new_width: {:?}, new_height: {:?}", new_width, new_height);

    let to_insert = new_width * rows * 2
                    + rows * config.tile_size * columns * 2;
    image.buffer.reserve_exact(to_insert);

    let mut inserted = 0;

    for i in 0..image.buffer.len()
    {
        let pixel_x = i % image.width;

        if pixel_x == 0 // at start of pixel row
        {
            let row = i / image.width;

            if row % config.tile_size == 0 // at start of tile row
            {
                if i == 0 // at first pixel row
                {
                    inserted += insert_pixels(&mut image.buffer, i+inserted, vec![Default::default(); new_width-1]);
                }
                else
                {
                    inserted += insert_pixels(&mut image.buffer, i+inserted, vec![Default::default(); new_width * 2]);
                }
            }

            inserted += insert_pixels(&mut image.buffer, i+inserted, vec![Default::default(); 2]);
        }
        else if pixel_x % config.tile_size == 0 // at start of tile column
        {
            inserted += insert_pixels(&mut image.buffer, i+inserted, vec![Default::default(); 2]);
        }
    }

    let pos = image.buffer.len();
    inserted += insert_pixels(&mut image.buffer, pos, vec![Default::default(); new_width+1]);

    println!("  Resized image, inserting {} pixels", inserted);

    assert!(inserted == to_insert);
    assert!(image.buffer.len() % new_width == 0);
    assert!(new_height == image.buffer.len() / new_width);

    image.height = new_height;
    image.width = new_width;

    //
    // Extrude tiles
    //

    let tile_size_with_gutters = config.tile_size + 2;
    let tile_pixel_max = tile_size_with_gutters - 1;

    println!("  Extruding {} ({}*{}) tiles into 1-pixel gutters", columns*rows, columns, rows);

    for tile_row_i in 0..rows
    {
        for tile_column_i in 0..columns
        {
            for tile_pixel_y in 0..tile_size_with_gutters
            {
                let pixel_y = tile_pixel_y + tile_row_i * tile_size_with_gutters;

                for tile_pixel_x in 0..tile_size_with_gutters
                {
                    let pixel_x = tile_pixel_x + tile_column_i * tile_size_with_gutters;

                    let from_i = match tile_pixel_y
                    {
                        0 => match tile_pixel_x
                        {
                            0 =>
                                Some(get_pixel_index(pixel_x+1, pixel_y+1, image.width)),
                            v if (v == tile_pixel_max) =>
                                Some(get_pixel_index(pixel_x-1, pixel_y+1, image.width)),
                            _ =>
                                Some(get_pixel_index(pixel_x, pixel_y+1, image.width))
                        },
                        v if (v == tile_pixel_max) => match tile_pixel_x
                        {
                            0 =>
                                Some(get_pixel_index(pixel_x+1, pixel_y-1, image.width)),
                            v if (v == tile_pixel_max) =>
                                Some(get_pixel_index(pixel_x-1, pixel_y-1, image.width)),
                            _ =>
                                Some(get_pixel_index(pixel_x, pixel_y-1, image.width))
                        },
                        _ => match tile_pixel_x
                        {
                            0 =>
                                Some(get_pixel_index(pixel_x+1, pixel_y, image.width)),
                            v if (v == tile_pixel_max) =>
                                Some(get_pixel_index(pixel_x-1, pixel_y, image.width)),
                            _ =>
                                None
                        }
                    };

                    if let Some(from_i) = from_i
                    {
                        let to_i = get_pixel_index(pixel_x, pixel_y, image.width);
                        let from = image.buffer[from_i].clone();
                        image.buffer[to_i] = from;
                    }
                }
            }
        }
    }

    //
    // Write to file
    //

    let mut output_path = input_path.clone();
    let mut output_name = output_path.file_stem().ok_or("Invalid path")?.to_os_string();
    output_name.push(config.output_suffix);
    output_name.push(".png");
    output_path.set_file_name(output_name);

    lodepng::encode32_file(&output_path, &image.buffer, image.width, image.height)?;

    println!("  Wrote {} pixels to {:?}", image.buffer.len(), OsString::from(output_path));

    return Ok(());
}

fn main()
{
    println!();

    let args: Vec<String> = env::args().skip(1).collect();
    match parse_args(&args)
    {
        Ok(config) =>
        {
            for i in 0..config.input_paths.len()
            {
                if let Err(e) = process_image(&config, i)
                {
                    eprintln!("Error: {}", e);
                }
            }
        },

        Err(e) =>
        {
            eprintln!("Error: {}", e);
        }
    };

}
