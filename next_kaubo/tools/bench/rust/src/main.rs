use std::time::Instant;
use std::fmt::Display;
use std::collections::HashMap;

fn fib_iter(n: i32) -> i32 {
    if n <= 1 { return n; }
    let (mut a, mut b) = (0, 1);
    for _ in 2..=n { let t = a + b; a = b; b = t; }
    b
}

fn mandelbrot_once() -> usize {
    let w = 500; let h = 500; let max_iter = 50;
    let xmin = -2.0; let xmax = 1.0;
    let ymin = -1.5; let ymax = 1.5;
    let dx = (xmax - xmin) / w as f64;
    let dy = (ymax - ymin) / h as f64;
    let mut outside = 0;
    for py in 0..h {
        let y0 = ymin + py as f64 * dy;
        for px in 0..w {
            let x0 = xmin + px as f64 * dx;
            let (mut x, mut y) = (0.0, 0.0);
            let mut bail = false;
            for _ in 0..max_iter {
                if x*x + y*y > 4.0 { bail = true; break; }
                y = 2.0*x*y + y0;
                x = x*x - y*y + x0;
            }
            if !bail { outside += 1; }
        }
    }
    outside
}

fn sieve_once(n: usize) -> usize {
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

fn pipeline_once() -> i64 {
    let mut total: i64 = 0;
    for x in 1..=100_000i64 {
        if x % 2 != 0 {
            let t = x * 3;
            if t % 7 == 0 { total += t; }
        }
    }
    total
}

fn list_push_once(n: usize) -> usize {
    let mut v = Vec::with_capacity(n);
    for i in 0..n { v.push(i); }
    v.len()
}

fn string_concat_once(n: usize) -> usize {
    let mut s = String::with_capacity(n + 10);
    for _ in 0..n { s.push('b'); }
    s.len()
}

fn json_access_once(n: usize) -> usize {
    let mut d: HashMap<&str, usize> = HashMap::new();
    d.insert("value", 0);
    for _ in 0..n {
        let v = *d.get("value").unwrap_or(&0);
        let mut new_d = HashMap::new();
        new_d.insert("value", v + 1);
        d = new_d;
    }
    *d.get("value").unwrap_or(&0)
}

fn closure_call_once(n: usize) -> usize {
    let f = |x: usize| x + 1;
    let mut total = 0;
    for i in 0..n { total ^= f(i); }
    total
}

fn nested_loop_once(n: usize) -> usize {
    let mut total = 0;
    for i in 0..n { for j in 0..n { total += i * j; } }
    total
}

fn fact_loop_once(n: usize) -> usize {
    fn fac(m: usize) -> usize { (1..=m).product() }
    (1..=n).map(|x| fac(x)).sum()
}

fn timed<F, R>(f: F, loops: usize) where F: Fn() -> R, R: Display {
    let _ = f();  // warmup
    let t0 = Instant::now();
    let mut result: usize = 0;
    for _ in 0..loops {
        let r = f();
        result ^= format!("{}", r).len();
        std::hint::black_box(&r);
    }
    let elapsed_ns = t0.elapsed().as_nanos();
    let avg_ns = elapsed_ns / loops as u128;
    println!("{}", avg_ns);
    std::hint::black_box(result);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let suite = args.get(1).map(|s| s.as_str()).unwrap_or("all");
    let loops: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);

    match suite {
        "fib"          => timed(|| fib_iter(40), loops),
        "mandelbrot"   => {
            let l = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(5);
            timed(|| mandelbrot_once(), l);
        }
        "sieve"        => timed(|| sieve_once(100_000), loops),
        "pipeline"     => timed(|| pipeline_once(), loops),
        "list_push"    => timed(|| list_push_once(100_000), loops),
        "string_concat"=> timed(|| string_concat_once(1000), loops),
        "json_access"  => timed(|| json_access_once(10000), loops),
        "closure_call" => timed(|| closure_call_once(100_000), loops),
        "loop"         => timed(|| nested_loop_once(200), loops),
        "fact"         => timed(|| fact_loop_once(12), loops),
        "all" => {
            let m_loops: usize = 5;
            timed(|| fib_iter(40), loops);
            timed(|| mandelbrot_once(), m_loops);
            timed(|| sieve_once(100_000), loops);
            timed(|| pipeline_once(), loops);
            timed(|| list_push_once(100_000), loops);
            timed(|| string_concat_once(1000), loops);
            timed(|| json_access_once(10000), loops);
            timed(|| closure_call_once(100_000), loops);
        }
        _ => eprintln!("Unknown suite: {}", suite),
    }
}
