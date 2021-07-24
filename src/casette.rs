use std::convert::TryInto;
use std::fmt;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::{Error, ErrorKind};

use ::image;
use piston_window::*;

#[derive(Default)]
pub struct Casette {
    pub prgrom_size: usize,
    pub chrrom_size: usize,
    pub prgrom: Vec<u8>,
    pub chrrom: Vec<u8>,
}

impl fmt::Debug for Casette {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let prgrom = format!("prgrom: {{ value: [...] }}");
        let chrrom = format!("chrrom: {{ value: [...] }}");

        write!(
            f,
            "Nes {{ prgrom_size: {:?}, chrrom_size: {:?}, {}, {} }}",
            self.prgrom_size, self.chrrom_size, prgrom, chrrom
        )
    }
}

impl Casette {
    pub fn load(path: &str) -> Result<Self, io::Error> {
        let mut f = File::open(path)?;
        // let mut buffer = Vec::new();

        let mut header = [0u8; 16];

        match f.read(&mut header) {
            Ok(read_size) => {
                if read_size != 16 {
                    return Err(Error::new(ErrorKind::Other, "Failed to read header"));
                }
            }
            Err(_) => {
                return Err(Error::new(ErrorKind::Other, "Failed to read header"));
            }
        }

        let magic =
            String::from_utf8(header[0..3].to_vec()).unwrap_or("Magic is wrong!".to_string());

        if magic != "NES" {
            return Err(Error::new(ErrorKind::Other, magic));
        }

        let prgrom_size = (header[4] as usize) * 1024 * 16;
        let chrrom_size = (header[5] as usize) * 1024 * 8;

        // println!("{}", prgrom_size);
        // println!("{}", chrrom_size);

        let mut prgrom = vec![0; prgrom_size];
        let mut chrrom = vec![0; chrrom_size];

        match f.read(&mut prgrom) {
            Ok(read_size) => {
                if read_size != prgrom_size {
                    return Err(Error::new(ErrorKind::Other, "Failed to read program area"));
                }
            }
            Err(_) => {
                return Err(Error::new(ErrorKind::Other, "Failed to read program area"));
            }
        }

        match f.read(&mut chrrom) {
            Ok(read_size) => {
                if read_size != chrrom_size {
                    return Err(Error::new(
                        ErrorKind::Other,
                        "Failed to read charactor rom area",
                    ));
                }
            }
            Err(_) => {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Failed to read charactor rom area",
                ));
            }
        }

        Ok(Casette {
            prgrom_size,
            chrrom_size,
            prgrom,
            chrrom,
        })
    }

    pub fn img(&self) -> Option<image::RgbaImage> {
        let num = self.chrrom.len() / 16;

        if num == 0 {
            return None;
        }

        // const UNIT: usize = 2;
        const DOT: usize = 8;

        let w = 50 as usize; //put 50 splits horizontally
        let h = num / w + (num % w != 0) as usize; //put h splits vertically

        let mut img: image::RgbaImage = image::ImageBuffer::new((w * DOT) as u32, (h * DOT) as u32);

        // img.put_pixel(x: u32, y: u32, pixel: P);

        const COLOR_PALLETTE: [[u8; 4]; 4] = [
            [0, 0, 0, 255],       //black
            [0, 0, 0, 255],       //black
            [0, 0, 0, 255],       //black
            [255, 255, 255, 255], //white
        ];

        // println!("{}", num);

        (0..num).for_each(|sprite_index| {
            let sprite: [u8; 16] = self
                .chrrom
                .get(sprite_index * 16..(sprite_index + 1) * 16)
                .unwrap()
                .try_into()
                .unwrap();

            let cindexes = Casette::calc_cindex(sprite);

            let row = sprite_index % w;
            let col = sprite_index / w;
            let xoffset = row * 8;
            let yoffset = col * 8;

            (0..8).for_each(|y| {
                let indexes = &cindexes[y * 8..(y + 1) * 8];
                indexes.into_iter().enumerate().for_each(|(x, c)| {
                    let pixel = image::Rgba(COLOR_PALLETTE[*c]);
                    img.put_pixel((x + xoffset) as u32, (y + yoffset) as u32, pixel);
                });
            });
        });

        Some(img)
    }

    pub fn show(&self) {
        const SCALE: u32 = 3;
        let img = match self.img() {
            Some(img) => img,
            None => image::ImageBuffer::new(200, 100),
        };

        let size = img.dimensions();

        let mut window: PistonWindow =
            WindowSettings::new("Hello Piston!", [size.0 * SCALE, size.1 * SCALE])
                .exit_on_esc(true)
                .vsync(true)
                .resizable(false)
                .samples(0)
                .build()
                .unwrap_or_else(|e| panic!("Failed to build PistonWindow: {}", e));

        let sprites = Texture::from_image(
            &mut window.create_texture_context(),
            &img,
            &TextureSettings::new(),
        )
        .unwrap();

        while let Some(event) = window.next() {
            match event {
                Event::Loop(Loop::Render(_)) => {
                    window.draw_2d(&event, |context, graphics, _device| {
                        clear([0.0; 4], graphics);
                        image(
                            &sprites,
                            context
                                .transform
                                .trans(0.0, 0.0)
                                .scale(SCALE as f64, SCALE as f64),
                            graphics,
                        );
                    });
                }
                _ => {}
            }
        }
    }

    pub fn read8(&self, offset: u16) -> u8 {
        match self.prgrom.get(offset as usize) {
            Some(data) => *data,
            None => 0x00,
        }
    }

    fn calc_cindex(sprite: [u8; 16]) -> [usize; 64] {
        let sprite1 = &sprite[0..8];
        let sprite2 = &sprite[8..16];
        let mut palette = [0usize; 64];

        sprite1
            .iter()
            .zip(sprite2)
            .enumerate()
            .for_each(|(i, (row1, row2))| {
                let odd_palette_num = (row1 & 0b0101_0101) | ((row2 & 0b0101_0101) << 1);
                let even_palette_num = ((row1 & 0b1010_1010) >> 1) | (row2 & 0b1010_1010);

                palette[i * 8 + 0] = ((even_palette_num & 0b1100_0000) >> 6) as usize;
                palette[i * 8 + 2] = ((even_palette_num & 0b0011_0000) >> 4) as usize;
                palette[i * 8 + 4] = ((even_palette_num & 0b0000_1100) >> 2) as usize;
                palette[i * 8 + 6] = ((even_palette_num & 0b0000_0011) >> 0) as usize;

                palette[i * 8 + 1] = ((odd_palette_num & 0b1100_0000) >> 6) as usize;
                palette[i * 8 + 3] = ((odd_palette_num & 0b0011_0000) >> 4) as usize;
                palette[i * 8 + 5] = ((odd_palette_num & 0b0000_1100) >> 2) as usize;
                palette[i * 8 + 7] = ((odd_palette_num & 0b0000_0011) >> 0) as usize;
            });

        palette
    }
}

#[test]
fn sprite_test() {
    let sprite = [
        0x66, 0x7F, 0xFF, 0xFF, 0xFF, 0x7E, 0x3C, 0x18, 0x66, 0x5F, 0xBF, 0xBF, 0xFF, 0x7E, 0x3C,
        0x18,
    ];

    let palette = Casette::calc_cindex(sprite);
    let heart = [
        0, 3, 3, 0, 0, 3, 3, 0, 0, 3, 1, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3,
        3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 0, 3, 3, 3, 3, 3, 3, 0, 0, 0, 3, 3, 3, 3, 0, 0, 0, 0, 0, 3,
        3, 0, 0, 0,
    ];
    println!("{:?}", palette);
    assert_eq!(palette, heart);
}

#[test]
fn save_img() {
    Casette::load("sample1.nes")
        .unwrap()
        .img()
        .unwrap()
        .save("sprite.png")
        .unwrap();
}
