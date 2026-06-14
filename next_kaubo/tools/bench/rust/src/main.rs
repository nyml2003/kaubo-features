use std::time::{Instant, Duration};

fn fib_iter(n: i32) -> i32 {
    if n <= 1 { return n; }
    let (mut a, mut b) = (0, 1);
    for _ in 2..=n { let t = a + b; a = b; b = t; }
    b
}

fn mandelbrot() -> &'static str {
    let w = 500; let h = 500; let max_iter = 50;
    let xmin = -2.0; let xmax = 1.0;
    let ymin = -1.5; let ymax = 1.5;
    let dx = (xmax - xmin) / w as f64;
    let dy = (ymax - ymin) / h as f64;
    for py in 0..h {
        let y0 = ymin + py as f64 * dy;
        for px in 0..w {
            let x0 = xmin + px as f64 * dx;
            let (mut x, mut y) = (0.0, 0.0);
            for _ in 0..max_iter {
                if x*x + y*y > 4.0 { break; }
                y = 2.0*x*y + y0;
                x = x*x - y*y + x0;
            }
        }
    }
    "ok"
}

fn sieve(n: usize) -> usize {
    let mut count = 0;
    for p in 2..=n {
        let mut is_prime = true; let mut d = 2;
        while d * d <= p {
            if p % d == 0 { is_prime = false; break; }
            d += 1;
        }
        if is_prime { count += 1; }
    }
    count
}

fn pipeline() -> i64 {
    let mut total: i64 = 0;
    for x in 1..=100_000i64 {
        if x % 2 != 0 {
            let t = x * 3;
            if t % 7 == 0 { total += t; }
        }
    }
    total
}

fn timed<F, R>(name: &str, f: F) where F: FnOnce() -> R {
    let s = Instant::now(); let r = f(); let e = s.elapsed();
    println!("rust_{} {}us", name, e.as_micros());
    println!("{}", r); // result on separate line for extraction
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let suite = args.get(1).map(|s| s.as_str()).unwrap_or("all");

    match suite {
        "fib" => timed("fib", || fib_iter(40)),
        "mandelbrot" => timed("mand", || mandelbrot()),
        "sieve" => timed("sieve", || sieve(100_000)),
        "pipeline" => timed("pipeline", || pipeline()),
        "all" => {
            timed("fib", || fib_iter(40));
            timed("mand", || mandelbrot());
            timed("sieve", || sieve(100_000));
            timed("pipeline", || pipeline());
        }
        _ => eprintln!("Unknown suite: {}", suite),
    }
}
