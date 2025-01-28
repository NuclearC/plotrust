extern crate sdl2;

use std::f64::consts::PI;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::libc::xdp_ring_offset;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};

const WINDOW_WIDTH: i32 = 800;
const WINDOW_HEIGHT: i32 = 800;

struct View {
    x: f64,
    y: f64,
    zoom: f64,
}
impl View {
    pub fn from(x: f64, y: f64, zoom: f64) -> View {
        View { x, y, zoom }
    }
}

fn view_to_window((x, y): (f64, f64), view: &View) -> (i32, i32) {
    let new_x = ((x - view.x) / view.zoom + 1.0) * ((WINDOW_WIDTH / 2) as f64);
    let new_y = ((y - view.y) / view.zoom + 1.0) * ((WINDOW_HEIGHT / 2) as f64);

    ((new_x as i32), (new_y as i32))
}

fn draw_line(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    start: (i32, i32),
    end: (i32, i32),
) {
    if (end.0 - start.0).abs() < 1 && (end.1 - start.1).abs() < 1 {
        return;
    }
    canvas
        .draw_line(Point::new(start.0, start.1), Point::new(end.0, end.1))
        .unwrap();
}

fn draw_rectangle(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    position: (f64, f64),
    side: f64,
    view: &View,
) {
    let left_top = view_to_window((position.0 - side, position.1 - side), view);
    let bottom_right = view_to_window((position.0 + side, position.1 + side), view);

    canvas
        .fill_rect(Rect::new(
            left_top.0 as i32,
            left_top.1 as i32,
            (bottom_right.0 - left_top.0).max(3) as u32,
            (bottom_right.1 - left_top.1).max(3) as u32,
        ))
        .unwrap();
}

fn draw_line2(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    start: (f64, f64),
    end: (f64, f64),
    view: &View,
) {
    draw_line(
        canvas,
        view_to_window(start, view),
        view_to_window(end, view),
    );
}

fn draw_grid(canvas: &mut sdl2::render::Canvas<sdl2::video::Window>, view: &View) {
    canvas.set_draw_color(Color::RGB(0x20, 0x20, 0x20));
    let real_grid_size = 1.0 / view.zoom;
    let mut grid_size: f64 = real_grid_size;
    while grid_size < 0.1 {
        grid_size *= 2.0;
    }
    while grid_size > 0.2 {
        grid_size /= 2.0;
    }

    let offset_y = ((view.y / view.zoom) % grid_size + grid_size) % grid_size;
    let offset_x = ((view.x / view.zoom) % grid_size + grid_size) % grid_size;

    for i in (-(1.0 / grid_size) as i32)..((1.0 / grid_size) as i32 + 1) {
        draw_line(
            canvas,
            view_to_window(
                (-1.0, (i as f64) * grid_size),
                &View::from(0.0, offset_y, 1.0),
            ),
            view_to_window(
                (1.0, (i as f64) * grid_size),
                &View::from(0.0, offset_y, 1.0),
            ),
        );
    }

    for i in (-(1.0 / grid_size) as i32)..((1.0 / grid_size) as i32 + 1) {
        draw_line(
            canvas,
            view_to_window(
                ((i as f64) * grid_size, -1.0),
                &View::from(offset_x, 0.0, 1.0),
            ),
            view_to_window(
                ((i as f64) * grid_size, 1.0),
                &View::from(offset_x, 0.0, 1.0),
            ),
        );
    }
}

fn draw_axis(canvas: &mut sdl2::render::Canvas<sdl2::video::Window>, view: &View) {
    canvas.set_draw_color(Color::RGB(0x40, 0x40, 0x40));
    draw_line(
        canvas,
        view_to_window((0.0, -1.0), &View::from(view.x / view.zoom, 0.0, 1.0)),
        view_to_window((0.0, 1.0), &View::from(view.x / view.zoom, 0.0, 1.0)),
    );
    draw_line(
        canvas,
        view_to_window((-1.0, 0.0), &View::from(0.0, view.y / view.zoom, 1.0)),
        view_to_window((1.0, 0.0), &View::from(0.0, view.y / view.zoom, 1.0)),
    );
}

fn draw_arrow(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    view: &View,
    pos: (f64, f64),
    slope: f64,
    length: f64,
) {
    let end = (pos.0 + length * slope.cos(), pos.1 + length * slope.sin());
    draw_line2(canvas, pos, end, &view);
    draw_rectangle(canvas, end, 0.01, view);
}

fn draw_function<F>(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    view: &View,
    f: F,
    start: f64,
    end: f64,
    steps: i32,
) where
    F: Fn(f64) -> f64,
{
    let mut last_pos: (f64, f64) = (start, f(start));
    let step_amount = (end - start) / (steps as f64);
    for i in 0..steps {
        let x = start + step_amount * (i as f64);
        let current_pos = (x, f(x));
        draw_line2(canvas, last_pos, current_pos, view);
        last_pos = current_pos;
    }
}

fn main() {
    let sdl_ctx = sdl2::init().unwrap();
    let sdl_vid = sdl_ctx.video().unwrap();

    let window = sdl_vid
        .window("sdl window", WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32)
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    let mut ev_pump = sdl_ctx.event_pump().unwrap();

    let mut view: View = View::from(0.0, 0.0, 1.0);

    let mut mov: (f64, f64) = (0.0, 0.0);

    'main_loop: loop {
        for event in ev_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    break 'main_loop;
                }
                Event::KeyUp {
                    timestamp: _,
                    window_id: _,
                    keycode,
                    ..
                } => match keycode.unwrap() {
                    Keycode::W => mov.1 = 0.0,
                    Keycode::A => mov.0 = 0.0,
                    Keycode::S => mov.1 = 0.0,
                    Keycode::D => mov.0 = 0.0,
                    _ => {}
                },
                Event::KeyDown {
                    timestamp: _,
                    window_id: _,
                    keycode,
                    ..
                } => match keycode.unwrap() {
                    Keycode::W => mov.1 = 0.0001,
                    Keycode::A => mov.0 = 0.0001,
                    Keycode::S => mov.1 = -0.0001,
                    Keycode::D => mov.0 = -0.0001,
                    Keycode::Escape => {
                        break 'main_loop;
                    }
                    _ => {}
                },
                Event::MouseWheel {
                    timestamp: _,
                    window_id: _,
                    which: _,
                    x,
                    y,
                    ..
                } => {
                    if y > 0 {
                        view.zoom = (view.zoom * 1.2).clamp(0.1, 10.0);
                    } else {
                        view.zoom = (view.zoom / 1.2).clamp(0.1, 10.0);
                    }
                }
                _ => {}
            }
        }

        view.x += mov.0;
        view.y += mov.1;

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        draw_grid(&mut canvas, &view);
        draw_axis(&mut canvas, &view);

        canvas.set_draw_color(Color::RGB(0xff, 0xff, 0));
        for ix in -10..10 {
            for iy in -10..10 {
                let x: f64 = ix as f64;
                let y: f64 = iy as f64;
                let sl: f64 = (x / PI).cos();
                draw_arrow(&mut canvas, &view, (x, y), (sl).atan(), 0.3);
            }
        }
        canvas.set_draw_color(Color::RGB(0, 0xff, 0));
        draw_function(&mut canvas, &view, |x| PI * (x / PI).sin(), -5.0, 5.0, 40);

        canvas.present();
    }
}
