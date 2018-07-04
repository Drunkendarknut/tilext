extern crate lodepng;

use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::ffi::OsString;

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
    TileSize,
    OutputSuffix,
    FilePath
}

fn parse_args<'a>(args: &'a Vec<String>) -> Result<Config<'a>, Box<Error>>
{
    let mut tile_size: Option<usize> = None;
    let mut output_suffix = "";
    let mut input_paths = Vec::<PathBuf>::new();
    let mut make_backup = false;

    use ArgsKey::*;
    let mut current_key = FilePath;
    for arg in args.as_slice()
    {
        match current_key
        {
            TileSize =>
            {
                tile_size = Some(arg.parse().map_err(
                    |e| format!("{} (after --tile--size)", e)
                )?);
                current_key = FilePath;
            },

            OutputSuffix =>
            {
                output_suffix = &arg;
                current_key = FilePath;
            },

            FilePath =>
            {
                if arg.starts_with("--")
                {
                    current_key = match &arg.chars().as_str()[2..]
                    {
                        "tile-size" => TileSize,
                        "output-suffix" => OutputSuffix,
                        s => {
                            println!("Warning: Argument key {} is unknown (ignoring)", s);
                            FilePath
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

    if current_key != FilePath
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

    return Ok(Config
    {
        tile_size: tile_size.ok_or("No tile size specified (use --tile-size)")?,

        output_suffix,

        make_backup,

        input_paths
    });
}

fn get_pixel_index(pixel_x: usize, pixel_y: usize, image_width: usize) -> usize
{
    pixel_y * image_width + pixel_x
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

    // TODO: Resize image, adding gutters

    //
    // Extrude tiles
    //

    let tile_size_with_gutters = config.tile_size + 2;
    let tile_max_x = tile_size_with_gutters - 1;
    let columns = image.width / tile_size_with_gutters;
    let rows = image.height / tile_size_with_gutters;

    println!("  Extruding {} ({}*{}) tiles into 1-pixel gutters", columns*rows, columns, rows);

    for tile_row_i in 0..rows {
        for tile_column_i in 0..columns {
            for tile_pixel_y in 0..tile_size_with_gutters {

                let pixel_y = tile_pixel_y + tile_row_i * tile_size_with_gutters;

                for tile_pixel_x in 0..tile_size_with_gutters {

                    let pixel_x = tile_pixel_x + tile_column_i * tile_size_with_gutters;

                    let from_i = if tile_pixel_y == 0 {

                        Some(
                            if tile_pixel_x == 0 {
                                get_pixel_index(pixel_x+1, pixel_y+1, image.width)
                            }
                            else if tile_pixel_x == tile_max_x {
                                get_pixel_index(pixel_x-1, pixel_y+1, image.width)
                            }
                            else {
                                get_pixel_index(pixel_x, pixel_y+1, image.width)
                            }
                        )
                    } else if tile_pixel_y == tile_max_x {

                        Some(
                            if tile_pixel_x == 0 {
                                get_pixel_index(pixel_x+1, pixel_y-1, image.width)
                            }
                            else if tile_pixel_x == tile_max_x {
                                get_pixel_index(pixel_x-1, pixel_y-1, image.width)
                            }
                            else {
                                get_pixel_index(pixel_x, pixel_y-1, image.width)
                            }
                        )
                    } else {

                        if tile_pixel_x == 0 {
                            Some(get_pixel_index(pixel_x+1, pixel_y, image.width))
                        }
                        else if tile_pixel_x == tile_size_with_gutters -1 {
                            Some(get_pixel_index(pixel_x-1, pixel_y, image.width))
                        }
                        else { None }
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
