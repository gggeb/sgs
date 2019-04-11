use std::io::{Read, Write, ErrorKind};
use std::net::TcpStream;
use std::num::ParseIntError;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

const WIDTH: i32 = 16;
const HEIGHT: i32 = 16;

type Point = (i32, i32);

fn deserialize_point(string: &str) -> Result<Point, ParseIntError> {
    let mut strings = [ String::new(), String::new() ];
    let mut index = 0;

    for c in string.chars() {
        if c == ':' {
            index = 1;
        } else {
            strings[index].push(c);
        }
    }

    Ok((strings[0].parse()?, strings[1].parse()?))
}

fn deserialize_points(string: &str) -> Result<Vec<Point>, ParseIntError> {
    let mut points = Vec::new();
    let mut buffer = String::new();

    for c in string.chars() {
        if c == ',' {
            points.push(deserialize_point(&buffer)?);
            buffer = String::new();
        } else {
            buffer.push(c);
        }
    }

    if buffer.len() > 0 { points.push(deserialize_point(&buffer)?); }

    Ok(points)
}

fn main() {
    let (mut x, mut y) = (0, 0);

    let mut stream = TcpStream::connect("0.0.0.0:63076")
        .expect("Unable to connect to server.");
    stream.set_nonblocking(true)
        .expect("Unable to make stream non-blocking.");

    let sdl_context = sdl2::init()
        .expect("Unable to create SDL2 context.");
    let video_subsystem = sdl_context.video()
        .expect("Unable to create video subsystem.");

    let window = video_subsystem.window("SGC", 512, 512)
        .position_centered()
        .build()
        .expect("Unable to create window.");

    let mut canvas = window.into_canvas().build()
        .expect("Unable to create canvas.");
    let mut event_pump = sdl_context.event_pump()
        .expect("Unable to create event pump.");

    let mut buffer = Vec::new();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Q), .. } => {
                    break 'running;
                },
                Event::KeyDown { keycode: Some(Keycode::Up), repeat: false, .. } => {
                    y -= 1;
                }
                Event::KeyDown { keycode: Some(Keycode::Right), repeat: false, .. } => {
                    x += 1;
                }
                Event::KeyDown { keycode: Some(Keycode::Down), repeat: false, .. } => {
                    y += 1;
                }
                Event::KeyDown { keycode: Some(Keycode::Left), repeat: false, .. } => {
                    x -= 1;
                }
                _ => {}
            }
        }

        if let Err(err) = stream.read_to_end(&mut buffer) {
            if let ErrorKind::WouldBlock = err.kind() {} else {
                println!("Disconnected from server.");
                break 'running;
            }
        }

        let string = String::from_utf8(buffer).unwrap();
        let states = string.split(";").collect::<Vec<_>>();
        buffer = Vec::new();

        for points in states {
            if points.len() > 0 {
                if let Ok(points) = deserialize_points(&points) {
                    canvas.set_draw_color(Color::RGB(255, 255, 255));
                    canvas.clear();

                    canvas.set_draw_color(Color::RGB(0, 0, 0));

                    canvas.fill_rects(&points.into_iter().map(|(x, y)| {
                        Rect::new(WIDTH * x, HEIGHT * y, WIDTH as u32, HEIGHT as u32)
                    }).collect::<Vec<_>>())
                        .expect("Unable to render points.");

                    break;
                }
            }
        }

        stream.write_all(&format!("{}:{}\n", x, y).as_bytes())
            .expect("Unable to send position.");

        canvas.present();
    }
}
