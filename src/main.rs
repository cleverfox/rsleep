use libc::{fd_set, select, timeval, FD_ISSET, FD_SET, FD_ZERO, STDIN_FILENO};
use std::io::{self, Write};
use std::mem;
// use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn parse_duration(arg: &str) -> Result<f64, String> {
    if arg.ends_with('d') {
        let num = arg[..arg.len() - 1]
            .parse::<f64>()
            .map_err(|_| "Invalid duration format")?;
        Ok(num * 86400.0)
    } else if arg.ends_with('h') {
        let num = arg[..arg.len() - 1]
            .parse::<f64>()
            .map_err(|_| "Invalid duration format")?;
        Ok(num * 3600.0)
    } else if arg.ends_with('m') {
        let num = arg[..arg.len() - 1]
            .parse::<f64>()
            .map_err(|_| "Invalid duration format")?;
        Ok(num * 60.0)
    } else {
        arg.parse::<f64>()
            .map_err(|_| "Invalid duration format".to_string())
    }
}

fn format_time(seconds: f64, width: usize) -> String {
    format!("{:0width$.1}s", seconds, width = width)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <duration>[s|m|h|d]", args[0]);
        eprintln!("Examples:");
        eprintln!("  {} 10      - sleep for 10 seconds", args[0]);
        eprintln!("  {} 1.5     - sleep for 1.5 seconds", args[0]);
        eprintln!("  {} 5m      - sleep for 5 minutes", args[0]);
        eprintln!("  {} 2h      - sleep for 2 hours", args[0]);
        eprintln!("  {} 1d      - sleep for 1 day", args[0]);
        std::process::exit(2);
    }

    let total_seconds = match parse_duration(&args[1]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(2);
        }
    };

    if total_seconds < 0.0 {
        eprintln!("Error: Duration must be positive");
        std::process::exit(2);
    }

    let is_terminal = atty::is(atty::Stream::Stdout);

    // Set up signal handler for Ctrl+C
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // Set up terminal for raw mode to detect 'q' key
    let mut raw_mode_enabled = false;
    if is_terminal {
        if let Err(_) = enable_raw_mode() {
            // If we can't enable raw mode, continue without it
        } else {
            raw_mode_enabled = true;
        }
    }

    let start = Instant::now();
    let total_duration = Duration::from_secs_f64(total_seconds);

    // Calculate padding width for whole numbers
    let max_width = format!("{:.1}", total_seconds).len();

    let exit_code = loop {
        let elapsed = start.elapsed();

        if elapsed >= total_duration {
            if is_terminal {
                let elapsed_s = total_seconds;
                let left_s = 0.0;
                print!(
                    "\r{}/{} left {}    \n",
                    format_time(elapsed_s, max_width),
                    format_time(total_seconds, max_width),
                    format_time(left_s, max_width)
                );
                io::stdout().flush().unwrap();
            }
            break 0;
        }

        if !running.load(Ordering::SeqCst) {
            if is_terminal {
                println!("\nInterrupted by Ctrl+C");
            }
            break 1;
        }

        // Check for 'q' key press
        if raw_mode_enabled && check_for_quit() {
            if is_terminal {
                println!("\nQuitting...");
            }
            break 0;
        }

        if is_terminal {
            let elapsed_s = elapsed.as_secs_f64();
            let left_s = total_seconds - elapsed_s;

            print!(
                "\r{}/{} left {}",
                format_time(elapsed_s, max_width),
                format_time(total_seconds, max_width),
                format_time(left_s, max_width)
            );
            io::stdout().flush().unwrap();
        }

        std::thread::sleep(Duration::from_millis(100));
    };

    if raw_mode_enabled {
        let _ = disable_raw_mode();
    }

    std::process::exit(exit_code);
}

// Terminal raw mode handling
fn enable_raw_mode() -> io::Result<()> {
    use libc::{tcgetattr, tcsetattr, termios, ECHO, ICANON, TCSANOW};
    use std::mem;
    use std::os::unix::io::AsRawFd;

    unsafe {
        let mut termios: termios = mem::zeroed();
        let stdin_fd = io::stdin().as_raw_fd();

        if tcgetattr(stdin_fd, &mut termios) != 0 {
            return Err(io::Error::last_os_error());
        }

        termios.c_lflag &= !(ICANON | ECHO);

        if tcsetattr(stdin_fd, TCSANOW, &termios) != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(())
}

fn disable_raw_mode() -> io::Result<()> {
    use libc::{tcgetattr, tcsetattr, termios, ECHO, ICANON, TCSANOW};
    use std::mem;
    use std::os::unix::io::AsRawFd;

    unsafe {
        let mut termios: termios = mem::zeroed();
        let stdin_fd = io::stdin().as_raw_fd();

        if tcgetattr(stdin_fd, &mut termios) != 0 {
            return Err(io::Error::last_os_error());
        }

        termios.c_lflag |= ICANON | ECHO;

        if tcsetattr(stdin_fd, TCSANOW, &termios) != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(())
}

fn check_for_quit() -> bool {
    unsafe {
        let mut fds: fd_set = mem::zeroed();
        FD_ZERO(&mut fds);
        FD_SET(STDIN_FILENO, &mut fds);

        let mut timeout = timeval {
            tv_sec: 0,
            tv_usec: 0,
        };

        let result = select(
            STDIN_FILENO + 1,
            &mut fds,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut timeout,
        );

        if result > 0 && FD_ISSET(STDIN_FILENO, &fds) {
            let mut buf = [0u8; 1];
            if libc::read(STDIN_FILENO, buf.as_mut_ptr() as *mut libc::c_void, 1) > 0 {
                return buf[0] == b'q' || buf[0] == b'Q';
            }
        }
    }

    false
}
