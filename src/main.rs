extern crate lodepng;

use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::ffi::OsString;

struct Config
{
    tile_size: usize,
    input_paths: Vec<PathBuf>
}

fn get_pixel_index(pixel_x: usize, pixel_y: usize, image_width: usize) -> usize
{
    pixel_y * image_width + pixel_x
}

fn process_image(config: &Config, path_i: usize) -> Result<(), Box<Error>>
{
    let input_path = &config.input_paths[path_i];

    println!("\nFile: {:?}:", input_path);

    //
    // Read file
    //

    let mut image = lodepng::decode32_file(input_path)?;

    println!("  Read {} pixels from file", image.buffer.len());

    //
    // Make backup
    //

    let mut file_name: OsString = input_path.file_name().ok_or("")?.to_os_string();
    let mut backup_path = input_path.clone();
    file_name.push(".backup");
    backup_path.set_file_name(file_name);

    lodepng::encode32_file(&backup_path, &image.buffer, image.width, image.height)?;

    println!("  Wrote {} pixels to {:?}", image.buffer.len(), OsString::from(backup_path));

    //
    // Extrude tiles
    //

    let tile_size_with_gutters = config.tile_size + 2;
    let columns = image.width / tile_size_with_gutters;
    let rows = image.height / tile_size_with_gutters;

    println!("  Extruding {} ({}*{}) tiles into 1-pixel gutters", columns*rows, columns, rows);

    for tile_row_i in 0..rows {
        for tile_column_i in 0..columns {
            for tile_pixel_y in 0..tile_size_with_gutters {

                let pixel_y = tile_pixel_y + tile_row_i * tile_size_with_gutters;

                for tile_pixel_x in 0..tile_size_with_gutters {

                    let pixel_x = tile_pixel_x + tile_column_i * tile_size_with_gutters;

                    let tile_max_x = tile_size_with_gutters - 1;

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

    lodepng::encode32_file(input_path, &image.buffer, image.width, image.height)?;

    println!("  Wrote {} pixels to {:?}", image.buffer.len(), OsString::from(input_path));

    return Ok(());
}

fn main()
{
    println!();

    //
    // Get arguments
    //

    let args: Vec<String> = env::args().collect();

    if args.len() < 3 { panic!("not enough arguments"); }

    let mut input_paths = Vec::<PathBuf>::new();
    for it in args[2..].iter().map(|s| PathBuf::from(s))
    {
        input_paths.push(it);
    }

    let config = Config {
        tile_size: args[1].parse()
            .expect("The first argument should be the width/height of one tile (exluding gutters). Ex: tilext 16 image.png"),
        input_paths
    };

    for i in 0..config.input_paths.len()
    {
        if let Err(e) = process_image(&config, i)
        {
            eprintln!("Error: {}", e);
        }
    }
}
