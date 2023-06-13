use std::env;
use std::process::exit;

const PROG_NAME: &str = env!("CARGO_BIN_NAME");
const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
const SECOND: u64 = 1;
const MINUTE: u64 = 60 * SECOND;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
const WEEK: u64 = 7 * DAY;
const MONTH: u64 = 30 * DAY;
const YEAR: u64 = 365 * DAY;
const PERIODS: &[u64] = &[SECOND, MINUTE, HOUR, DAY, WEEK, MONTH, YEAR];
const PERIOD_NAMES: &[&str] = &["sec", "min", "hour", "day", "week", "month", "year"];

fn main() {
    let mut pargs = pico_args::Arguments::from_env();
    if pargs.contains(["-h", "--help"]) || env::args().len() == 1 {
        println!("Usage: {} <number> <unit> / <period>", PROG_NAME);
        println!("       <number>: integer or float (no scientific notation)");
        println!("       <unit>  : {}", UNITS.join(" "));
        println!("       <period>: {}", PERIOD_NAMES.join(" "));
        exit(0);
    }
    if pargs.contains(["-v", "--version"]) {
        println!("{} {}", PROG_NAME, env!("CARGO_PKG_VERSION"));
        exit(0);
    }

    let mut s = String::new();
    let mut sep: &str = "";
    for a in env::args().skip(1) {
        s.push_str(sep);
        s.push_str(a.as_str());
        sep = " ";
    }
    match parse(&s) {
        Ok(bytes_per_second) => {
            for i in 0..PERIODS.len() {
                let period = PERIODS[i];
                let period_name = PERIOD_NAMES[i];
                let (rate, unit) = nearest_power_of_1000_rate(bytes_per_second * period as f64);
                println!("{:>7.3?} {:>2} / {}", rate, unit, period_name);
            }
        }
        Err(e) => {
            eprintln!("{}: {}", PROG_NAME, e);
            exit(1);
        }
    }
}

fn nearest_power_of_1000_rate(mut bytes: f64) -> (f64, &'static str) {
    for unit in UNITS {
        if bytes < 1000.0 {
            return (bytes, unit);
        }
        bytes /= 1000.0;
    }
    // If you didn't fit in yottabytes, you might as well be considered infinite.
    return (f64::INFINITY, "B");
}

fn period_to_seconds(period_name: &str) -> Result<u64, ParseError> {
    match period_name {
        "s" | "sec" | "second" => Ok(SECOND),
        "m" | "min" | "minute" => Ok(MINUTE),
        "h" | "hr" | "hour" => Ok(HOUR),
        "d" | "day" => Ok(DAY),
        "w" | "wk" | "week" => Ok(WEEK),
        "mon" | "month" => Ok(MONTH),
        "y" | "yr" | "year" => Ok(YEAR),
        _ => Err(ParseError::InvalidPeriod),
    }
}

#[derive(Debug, Eq, PartialEq)]
enum ParseError {
    InvalidNumber,
    InvalidUnit,
    InvalidPeriod,
    UnexpectedCharacter { expected: u8, actual: u8 },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidNumber => write!(f, "not a valid number"),
            Self::InvalidUnit => write!(f, "not a recognized unit ({})", UNITS.join(" ")),
            Self::InvalidPeriod => write!(
                f,
                "not a recognized time period ({})",
                PERIOD_NAMES.join(" ")
            ),
            Self::UnexpectedCharacter { expected, actual } => {
                write!(
                    f,
                    "expected {:?}, found {:?}",
                    *expected as char, *actual as char
                )
            }
        }
    }
}

fn parse(s: &str) -> Result<f64, ParseError> {
    let mut p = Parser {
        buf: s.as_bytes(),
        pos: 0,
    };
    p.skip_whitespace();
    let rate: f64 = p.parse_number()?;
    p.skip_whitespace();
    let byte_multiplier: f64 = p.parse_bytes()?;
    p.skip_whitespace();
    p.expect(b'/')?;
    p.skip_whitespace();
    let seconds: f64 = p.parse_period()?;
    return Ok(rate * byte_multiplier / seconds);
}

struct Parser<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> u8 {
        if self.pos >= self.buf.len() {
            return 0;
        }
        return self.buf[self.pos];
    }

    fn eof(&self) -> bool {
        return self.peek() == 0;
    }

    fn advance(&mut self) -> u8 {
        let b = self.peek();
        if b == 0 {
            return b;
        }
        self.pos += 1;
        return b;
    }

    fn expect(&mut self, expected: u8) -> Result<u8, ParseError> {
        let actual = self.advance();
        if actual == expected {
            return Ok(actual);
        }
        return Err(ParseError::UnexpectedCharacter { expected, actual });
    }

    fn skip_whitespace(&mut self) {
        while !self.eof() && self.peek().is_ascii_whitespace() {
            self.advance();
        }
    }

    fn parse_number(&mut self) -> Result<f64, ParseError> {
        let start_pos = self.pos;
        while !self.eof() && self.peek().is_ascii_digit() {
            self.advance();
        }
        // No digits
        // NB(vincent): shouldn't ever trigger, we get inside `parse_number`
        // because we saw a digit. Still gonna put it for good measure.
        if start_pos == self.pos {
            return Err(ParseError::InvalidNumber);
        }

        if self.peek() == b'.' {
            self.advance(); // eat the '.'
            let decimals_start = self.pos;
            while !self.eof() && self.peek().is_ascii_digit() {
                self.advance();
            }
            if decimals_start == self.pos {
                return Err(ParseError::InvalidNumber);
            }
        }

        let s = unsafe { std::str::from_utf8_unchecked(&self.buf[start_pos..self.pos]) };
        match s.parse::<f64>() {
            Ok(x) => return Ok(x),
            Err(_) => return Err(ParseError::InvalidNumber),
        }
    }

    /// Parses strings like "B", "MB", "TB", etc. and returns how many
    /// bytes that it (e.g., "B" -> 1, "MB" -> 1e6, "TB" -> 1e12).
    fn parse_bytes(&mut self) -> Result<f64, ParseError> {
        let start_pos = self.pos;
        while !self.eof() && self.peek().is_ascii_alphabetic() {
            self.advance();
        }
        let unit = (&self.buf[start_pos..self.pos]).to_ascii_uppercase();
        for (i, candidate) in UNITS.iter().enumerate() {
            if candidate.as_bytes() == unit {
                return Ok(f64::powf(1000.0, i as f64));
            }
        }
        return Err(ParseError::InvalidUnit);
    }

    fn parse_period(&mut self) -> Result<f64, ParseError> {
        let start_pos = self.pos;
        while !self.eof() && self.peek().is_ascii_alphabetic() {
            self.advance();
        }
        let period = unsafe { std::str::from_utf8_unchecked(&self.buf[start_pos..self.pos]) };
        let period = period.to_ascii_lowercase();
        let seconds = period_to_seconds(&period)? as f64;
        return Ok(seconds);
    }
}

#[test]
fn test_parse_whitespaces() {
    // See that we can put whitespaces pretty much everywhere
    assert!(parse("1B/s").is_ok());
    assert!(parse("1 B/s").is_ok());
    assert!(parse("1B /s").is_ok());
    assert!(parse("1B/ s").is_ok());
    assert!(parse("1B / s").is_ok());
    assert!(parse("1 B/ s").is_ok());
    assert!(parse("1 B / s").is_ok());
    assert!(parse(" 1 B / s ").is_ok());
}

#[test]
fn test_parse_units() {
    // Try the different units and with different casing.
    assert!(parse("1 b / s").is_ok());
    assert!(parse("1 B / s").is_ok());
    assert!(parse("1 kB / s").is_ok());
    assert!(parse("1 Kb / s").is_ok());
    assert!(parse("1 KB / s").is_ok());
    assert!(parse("1 MB / s").is_ok());
    assert!(parse("1 GB / s").is_ok());
    assert!(parse("1 TB / s").is_ok());
    assert!(parse("1 PB / s").is_ok());
    assert!(parse("1 EB / s").is_ok());
    assert!(parse("1 ZB / s").is_ok());
    assert!(parse("1 YB / s").is_ok());
}

#[test]
fn test_parse_periods() {
    // Try the different period spellings and with different casing.
    assert!(parse("1 B / s").is_ok());
    assert!(parse("1 B / S").is_ok());
    assert!(parse("1 B / sec").is_ok());
    assert!(parse("1 B / SEC").is_ok());
    assert!(parse("1 B / SeC").is_ok());
    assert!(parse("1 B / SEC").is_ok());
    assert!(parse("1 B / second").is_ok());
    assert!(parse("1 B / m").is_ok());
    assert!(parse("1 B / min").is_ok());
    assert!(parse("1 B / minute").is_ok());
    assert!(parse("1 B / h").is_ok());
    assert!(parse("1 B / hr").is_ok());
    assert!(parse("1 B / hour").is_ok());
    assert!(parse("1 B / d").is_ok());
    assert!(parse("1 B / day").is_ok());
    assert!(parse("1 B / w").is_ok());
    assert!(parse("1 B / wk").is_ok());
    assert!(parse("1 B / week").is_ok());
    assert!(parse("1 B / mon").is_ok());
    assert!(parse("1 B / month").is_ok());
    assert!(parse("1 B / y").is_ok());
    assert!(parse("1 B / yr").is_ok());
    assert!(parse("1 B / year").is_ok());
}

#[test]
fn test_parse_invalid_inputs() {
    // All sorts of invalid input. I probably cannot think of
    // all the weird-ass ways to get an error though.
    assert!(parse("").is_err());
    assert!(parse("1").is_err());
    assert!(parse("1B").is_err());
    assert!(parse("1B/").is_err());
    assert!(parse("1Bps").is_err());
    assert!(parse("1Bs").is_err());
    assert!(parse("1B:s").is_err());

    assert!(parse("x MB/s").is_err());
    assert!(parse("1e7 MB/s").is_err());
    assert!(parse("-33 MB/s").is_err());
    assert!(parse("192.168.1.1 MB/s").is_err());
    assert!(parse("４ MB/s").is_err()); // wide digits

    assert!(parse("4 XB/s").is_err());
    assert!(parse("4 ML/s").is_err());
    assert!(parse("4 MMMB/s").is_err());
}

#[rustfmt::skip]
#[test]
fn test_parse_number() {
    let mut p = Parser { buf: b"", pos: 0 };
    assert!(p.parse_number().is_err());

    let mut p = Parser { buf: b"x", pos: 0 };
    assert!(p.parse_number().is_err());

    let mut p = Parser { buf: b"1", pos: 0 };
    assert_eq!(p.parse_number(), Ok(1.0));

    let mut p = Parser { buf: b"123", pos: 0 };
    assert_eq!(p.parse_number(), Ok(123.0));

    let mut p = Parser { buf: b"1.", pos: 0 };
    assert!(p.parse_number().is_err());

    let mut p = Parser { buf: b"1.25", pos: 0 };
    assert_eq!(p.parse_number(), Ok(1.25));
}

#[test]
fn test_parse_unit() {
    let mut p = Parser { buf: b"B", pos: 0 };
    assert_eq!(p.parse_bytes(), Ok(1.0));

    let mut p = Parser { buf: b"KB", pos: 0 };
    assert_eq!(p.parse_bytes(), Ok(1e3));

    let mut p = Parser { buf: b"MB", pos: 0 };
    assert_eq!(p.parse_bytes(), Ok(1e6));

    let mut p = Parser { buf: b"GB", pos: 0 };
    assert_eq!(p.parse_bytes(), Ok(1e9));

    let mut p = Parser { buf: b"TB", pos: 0 };
    assert_eq!(p.parse_bytes(), Ok(1e12));

    let mut p = Parser { buf: b"PB", pos: 0 };
    assert_eq!(p.parse_bytes(), Ok(1e15));

    let mut p = Parser { buf: b"EB", pos: 0 };
    assert_eq!(p.parse_bytes(), Ok(1e18));

    let mut p = Parser { buf: b"ZB", pos: 0 };
    assert_eq!(p.parse_bytes(), Ok(1e21));

    let mut p = Parser { buf: b"YB", pos: 0 };
    assert_eq!(p.parse_bytes(), Ok(1e24));
}
