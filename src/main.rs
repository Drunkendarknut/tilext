extern crate lodepng;

use std::env;
use std::path::PathBuf;
use std::ffi::OsString;

struct Config<'a>
{
    input_path: &'a PathBuf,
    tile_size: usize
}

fn main()
{
    //
    // Get arguments
    //

    let args: Vec<String> = env::args().collect();

    if args.len() < 3 { panic!("not enough arguments"); }

    // let file_name = args[1].clone();
    let config = Config {
        input_path: &PathBuf::from(&args[1]),
        tile_size: args[2].parse()
            .expect("The second argument should be tile width/height (exluding gutters). Ex: tilext image.png 16")
    };

    //
    // Read file
    //

    let mut image = lodepng::decode32_file(config.input_path).expect("couldn't open file");
    let bytes_per_pixel = std::mem::size_of::<lodepng::RGBA>(); // decode32_file always returns Bitmap<RGBA> data

    println!("\nRead {:?} bytes from {:?}\n", image.buffer.len() * bytes_per_pixel, OsString::from(config.input_path));

    //
    // Make backup
    //

    let mut backup_path = config.input_path.clone();
    let mut file_name = backup_path.file_name().unwrap().to_os_string();
    file_name.push(".backup");
    backup_path.set_file_name(file_name);

    lodepng::encode32_file(&backup_path, &image.buffer, image.width, image.height).expect("couldn't write to image");

    println!("Wrote {:?} bytes to {:?}\n", image.buffer.len() * bytes_per_pixel, OsString::from(backup_path));

    //
    // Extrude tiles
    //
    let tile_size_with_gutters = config.tile_size + 2;
    let columns = image.width / tile_size_with_gutters;
    let rows = image.height / tile_size_with_gutters;

    // for each row
    for tile_row_i in 0..rows {
        // for each column
        for tile_column_i in 0..columns {

            // get tile origin

            // for each pixel row in tile
            for tile_pixel_y in 0..tile_size_with_gutters {

                let pixel_y = tile_pixel_y + tile_column_i * tile_size_with_gutters;

                // for each pixel in row
                for tile_pixel_x in 0..tile_size_with_gutters {

                    let pixel_x = tile_pixel_x + tile_row_i * tile_size_with_gutters;

                    let pixel_i = pixel_y * image.width + pixel_x;

                    let from_i: Option<usize> =
                        // if top gutter
                        if tile_pixel_y == 0 {
                            Some(
                                // if left gutter
                                if tile_pixel_x == 0 {
                                    // copy from (1,1)
                                    (pixel_y + 1) * image.width + (pixel_x + 1)
                                }
                                // if right gutter
                                else if tile_pixel_x == tile_size_with_gutters - 1 {
                                    // copy from tile (-1,1)
                                    (pixel_y + 1) * image.width + (pixel_x - 1)
                                }
                                else {
                                    // copy from pixel below
                                    (pixel_y + 1) * image.width + pixel_x
                                } as usize
                            )
                        }
                        // if bottom gutter
                        else if tile_pixel_y == tile_size_with_gutters - 1
                        {
                            Some(
                                // if left gutter
                                if tile_pixel_x == 0 {
                                    // copy from (1,-1)
                                    (pixel_y - 1) * image.width + (pixel_x + 1)
                                }
                                // if right gutter
                                else if tile_pixel_x == tile_size_with_gutters - 1 {
                                    // copy from (-1,-1)
                                    (pixel_y - 1) * image.width + (pixel_x - 1)
                                }
                                else {
                                    // copy from pixel above
                                    (pixel_y - 1) * image.width + pixel_x
                                } as usize
                            )
                        }
                        else
                        {
                            // if left gutter
                            if tile_pixel_x == 0 {
                                // copy from right
                                Some((pixel_y * image.width + (pixel_x + 1)) as usize)
                            }
                            // if right gutter
                            else if tile_pixel_x == tile_size_with_gutters -1 {
                                // copy from left
                                Some((pixel_y * image.width + (pixel_x - 1)) as usize)
                            }
                            else { None }
                        };

                    if let Some(from_i) = from_i
                    {
                        let from = image.buffer[from_i].clone();
                        image.buffer[pixel_i] = from;
                    }

                }
            }
        }
    }

    //
    // Write to file
    //

    lodepng::encode32_file(config.input_path, &image.buffer, image.width, image.height).expect("couldn't write to image");

    println!("Wrote {:?} bytes to {:?}\n", image.buffer.len() * bytes_per_pixel, OsString::from(config.input_path));
}
