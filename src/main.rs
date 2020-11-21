extern crate piston_window;

use piston_window::*;
use std::{thread, time};
use std::sync::mpsc;
use std::net::{TcpStream};
use std::io::{self, Read, Write};
use std::str::from_utf8;
use std::sync::mpsc::TryRecvError;

// returns an rgba array from integer input using a predefined colour scheme
fn process_colour(input_colour:i32) -> [f32; 4] {
    match input_colour {
        0 => [0.0, 0.0, 0.25, 1.0],
        1 => [0.0, 1.0, 0.0, 1.0],
        2 => [1.0, 0.0, 0.0, 1.0],
        _ => [0.0, 0.0, 0.0, 1.0],
    }
}

fn process_mouse_click(state: &mut [[i32; 7]; 6], mouse_coords: [f64; 2]) -> (bool, u8) {
    let mut click_indexes: [i32; 2] = [-1, -1];
    let mut valid_click = true;

    // determine indexes of mouse click
    for coord_index in 0..mouse_coords.len() {
        // remove offset (subtract 5)
        let coord: i32 = (mouse_coords[coord_index] as i32) - 5;

        if coord % 55 > 50 {
            // if modulo 55 is greater than 50, no click
            valid_click = false;
        } else {
            // otherwise, get index of click by integer dividing by 55
            click_indexes[coord_index] = coord / 55;

            // check whether click is within the playable grid (7 by 6)
            if !(0..[7, 6][coord_index]).contains(&click_indexes[coord_index]){
                valid_click = false;
            }
        }
    }

    //println!("lmb pressed at {:?}", mouse_coords);
    //println!("indexes are {:?}", click_indexes);
    //println!("valid? {}", valid_click);

    if valid_click {
        (true, click_indexes[0] as u8)
    } else {
        (false, 0)
    }
}

fn main() {
    // holds the state of the game board
    let mut state = [[0; 7]; 6];
    // holds the latest mouse co-ordinates
    let mut mouse_coords = [0.0; 2];

    let (tx_server_client, rx_server_client) = mpsc::channel();
    let (tx_client_canvas, rx_client_canvas) = mpsc::channel();

    // launch networking thread
    thread::spawn(move || {
        // stores data received from stream
        let mut data = [0 as u8; 3];

        // server ip: 165.232.32.238
        // local testing: localhost
        match TcpStream::connect("localhost:32032") {
            Ok(mut network_stream) => {
                println!("Successfully connected to server in port 32032");

                // find out whether this is player 1 or 2
                let mut self_id = [0 as u8; 1];
                match network_stream.read_exact(&mut self_id) {
                    Ok(_) => {
                        let text = from_utf8(&self_id).unwrap();
                        println!("My self id: {}", text);
                    },
                    Err(e) => {
                        println!("Failed to receive data from network (0): {}", e);
                    }
                }

                if self_id[0] == 49 {
                    // clear the stream!
                    loop {
                        match rx_server_client.try_recv() {
                            Ok(..) => {},
                            Err(..) => {break},
                        }
                    }

                    let msg: [u8; 1] = [rx_server_client.recv().unwrap()];
                    //println!("Got: {:?}", msg);
                    network_stream.write(&msg).unwrap();

                    data = [0 as u8; 3];
                    match network_stream.read_exact(&mut data) {
                        Ok(_) => {
                            tx_client_canvas.send(data);
                        },
                        Err(e) => {
                            println!("Failed to receive data from network (1): {}", e);
                        }
                    }
                }

                loop {
                    data = [0 as u8; 3];
                    match network_stream.read_exact(&mut data) {
                        Ok(_) => {
                            tx_client_canvas.send(data);
                        },
                        Err(e) => {
                            println!("Failed to receive data from network (2): {}", e);
                        }
                    }

                    // clear the stream!
                    loop {
                        match rx_server_client.try_recv() {
                            Ok(T) => {},
                            Err(E) => {break},
                        }
                    }

                    let msg: [u8; 1] = [rx_server_client.recv().unwrap()];
                    //println!("Got: {:?}", msg);
                    network_stream.write(&msg).unwrap();

                    data = [0 as u8; 3];
                    match network_stream.read_exact(&mut data) {
                        Ok(_) => {
                            // TODO: clean this up
                            //let text = from_utf8(&data).unwrap();
                            //println!("RECEIVED MESSAGE FROM OTHER PLAYER: {}", text);
                            tx_client_canvas.send(data);
                        },
                        Err(e) => {
                            println!("Failed to receive data from network (3): {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("Failed to connect: {}", e);
            }
        }
        println!("Terminated.");
    });

    // set up Piston Window
    let mut window: PistonWindow =
        WindowSettings::new("Connect 4", [640, 480])
            .exit_on_esc(true).build().unwrap();

    // update/event loop - this seems to run regardless of user input
    while let Some(event) = window.next() {
        // handle updating of mouse co-ordinates
        match event.mouse_cursor_args() {
            Some(coords) => mouse_coords = coords,
            None => {},
        }

        // handle mouse click
        if let Some(Button::Mouse(button)) = event.press_args() {
            if button == MouseButton::Left {
                let (valid_click, click_column) = process_mouse_click(&mut state, mouse_coords);
                if valid_click && state[0][click_column as usize]==0 {
                    tx_server_client.send(click_column).unwrap();
                }
            }
        }

        // handle incoming state change
        match rx_client_canvas.try_recv() {
            // if there is a value waiting in the stream, act on it (modify state)
            Ok(T) => {
                state[T[0] as usize][T[1] as usize] = T[2] as i32;
                println!("Got new instruction {:?}", T);
            },
            // if not, do nothing
            Err(E) => {},
        }

        // draw window and objects/sprites
        window.draw_2d(&event, |context, graphics, _device| {
            // fill window with background colour
            clear([1.0; 4], graphics);

            // cycle through elements of state array
            for row_index in 0..(state.len()) {
                for column_index in 0..(state[0].len()) {
                    // calculate origins of this element
                    let x_origin = 5.0 + (column_index as f64) * 55.0;
                    let y_origin = 5.0 + (row_index as f64) * 55.0;

                    // draw this element in the correct colour, with a fixed size
                    rectangle(process_colour(state[row_index][column_index]),
                              [x_origin, y_origin, 50.0, 50.0],
                              context.transform,
                              graphics);
                }
            }
        });
    }
}