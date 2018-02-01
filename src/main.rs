extern crate image;
extern crate num;
extern crate time;
extern crate crossbeam;

use time::PreciseTime;
use num::Complex;
use std::io::Write;
use std::fs::File;
use std::str::FromStr;
use image::ColorType;
use image::png::PNGEncoder;

fn render(pixels: &mut [u8], bounds: (usize, usize), upper_left: Complex<f64>, lower_right: Complex<f64>) {
    assert!(pixels.len() == bounds.0 * bounds.1);

    for row in 0 .. bounds.1 {
        for column in 0 .. bounds.0 {
            let point = pixel_to_point(bounds, (column, row), upper_left, lower_right);
            pixels[row * bounds.0 + column] = 
                match escape_time(point, 255) {
                    None => 0,
                    Some(count) => 255 - count as u8
                };
        }
    }
}

fn parse_complex(s : &str) -> Option<Complex<f64>> {
    match parse_pair(s, ',') {
        Some((re, im)) => Some(Complex { re, im }),
        None => None
    }
}

fn pixel_to_point(bounds: (usize, usize), pixel: (usize, usize), upper_left: Complex<f64>, lower_right: Complex<f64>) -> Complex<f64> {
    let (width, height) = (lower_right.re - upper_left.re, upper_left.im - lower_right.im);

    Complex {
        re: upper_left.re + pixel.0 as f64 * width / bounds.0 as f64,
        im: upper_left.im - pixel.1 as f64 * height / bounds.1 as f64
    }
}


fn parse_pair<T: FromStr>(s: &str, seperator: char) -> Option<(T, T)>{
    match s.find(seperator) {
        None => None,
        Some(index) => {
            match (T::from_str(&s[..index]), T::from_str(&s[index+1 ..])) {
                (Ok(l), Ok(r)) => Some((l,r)),
                _ => None
            }
        }
    }

}

fn escape_time(c: Complex<f64>, limit: u32) -> Option<u32> {
	let mut z = Complex { re: 0.0, im: 0.0 };
	for i in 0..limit {
		z = z * z +  c;
		if z.norm_sqr() > 4.0 {
			return Some(i);
		}
	}
	None
}

fn write_image(filename: &str, pixels:&[u8], bounds: (usize, usize)) -> Result<(), std::io::Error> {
    let output = File::create(filename)?;

    let encoder = PNGEncoder::new(output);
    encoder.encode(&pixels, bounds.0 as u32, bounds.1 as u32, ColorType::Gray(8))?;

    Ok(())
}

fn main() {

    let start = PreciseTime::now();

    let args: Vec<String> = std::env::args().collect();

    if args.len() != 5 {
    	writeln!(std::io::stderr(),
    		"Usage: mandlebrot FILE PIXELS UPPERLEFT LOWERRIGHT")
    	.unwrap();
    	writeln!(std::io::stderr(), "Example {}", args[0] ).unwrap();
        std::process::exit(1);
    }

    let bounds = parse_pair(&args[2], 'x').expect("error parsing image dimensions");
    let upper_left = parse_complex(&args[3]).expect("error parsing upper left corner ");
    let lower_right = parse_complex(&args[4]).expect("error parsing lower right corner");

    let mut pixels = vec![0; bounds.0 * bounds.1];

    //render(&mut pixels, bounds, upper_left, lower_right);

    let threads = 8;
    let rows_per_band = bounds.1 / threads + 1;

    {
        let bands: Vec<&mut [u8]> = pixels.chunks_mut(rows_per_band * bounds.0).collect();
        crossbeam::scope(|spawner| {
            for (i, band) in bands.into_iter().enumerate(){
                let top = rows_per_band * i;
                let height = band.len() / bounds.0;
                let band_bounds = (bounds.0, height);
                let band_upper_left = pixel_to_point(bounds, (0, top), upper_left, lower_right);
                let band_lower_right = pixel_to_point(bounds, (bounds.0, top + height), upper_left, lower_right);

                spawner.spawn(move || {
                    render(band, band_bounds, band_upper_left, band_lower_right);
                });
            }
        });
    }

    write_image(&args[1], &pixels, bounds).expect("error writing PNG file");
    let end = PreciseTime::now();

    println!("{} seconds", start.to(end))
}