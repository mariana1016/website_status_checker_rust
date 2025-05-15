use std::{
    env,
    fs::File,
    io::{self, BufRead, Write},
    path::PathBuf,
    sync::{
        mpsc::{channel},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime},
};

use reqwest::blocking::Client;
use serde_json::json;

// WebsiteStatus structure
#[derive(Debug, Clone)]
struct WebsiteStatus {
    url: String,
    action_status: Result<u16, String>,
    response_time: Duration,
    timestamp: SystemTime,
}

// Config for command line args
struct Config {
    file_path: Option<PathBuf>,
    urls: Vec<String>,
    workers: usize,
    timeout: u64,
    retries: u32,
}

fn parse_arguments() -> Result<Config, String> {
    let mut args = env::args().skip(1);
    let mut config = Config {
        file_path: None,
        urls: Vec::new(),
        workers: num_cpus::get(),
        timeout: 5,
        retries: 0,
    };

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--file" => {
                if let Some(path_str) = args.next() {
                    config.file_path = Some(PathBuf::from(path_str));
                } else {
                    return Err("Error: Missing path for --file argument.".to_string());
                }
            }
            "--workers" => {
                if let Some(worker_str) = args.next() {
                    if let Ok(n) = worker_str.parse::<usize>() {
                        if n == 0 {
                            return Err("Error: --workers value must be greater than 0".to_string());
                        }
                        config.workers = n;
                    } else {
                        return Err("Error: Invalid value for --workers. Must be a positive integer.".to_string());
                    }
                } else {
                    return Err("Error: Missing value for --workers argument.".to_string());
                }
            }
            "--timeout" => {
                if let Some(timeout_str) = args.next() {
                    if let Ok(s) = timeout_str.parse::<u64>() {
                        config.timeout = s;
                    } else {
                        return Err("Error: Invalid value for --timeout. Must be a positive integer.".to_string());
                    }
                } else {
                    return Err("Error: Missing value for --timeout argument.".to_string());
                }
            }
            "--retries" => {
                if let Some(retries_str) = args.next() {
                    if let Ok(n) = retries_str.parse::<u32>() {
                        config.retries = n;
                    } else {
                        return Err("Error: Invalid value for --retries. Must be a non-negative integer.".to_string());
                    }
                } else {
                    return Err("Error: Missing value for --retries argument.".to_string());
                }
            }
            _ if arg.starts_with("--") => {
                return Err(format!("Error: Unknown argument: {}", arg));
            }
            url => config.urls.push(url.to_string()),
        }
    }

    if config.file_path.is_none() && config.urls.is_empty() {
        eprintln!("Usage: website_checker [--file <path>] [suspicious link removed] [--workers N] [--timeout S] [--retries N]");
        std::process::exit(2);
    }

    Ok(config)
}

fn read_urls_from_file(path: &PathBuf) -> Result<Vec<String>, io::Error> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);
    let mut urls = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let trimmed_line = line.trim();
        if !trimmed_line.is_empty() && !trimmed_line.starts_with('#') {
            urls.push(trimmed_line.to_string());
        }
    }
    Ok(urls)
}

fn check_website(url: String, timeout: Duration, retries: u32) -> WebsiteStatus {
    let client = match Client::builder().timeout(timeout).build() {
        Ok(client) => client,
        Err(e) => return WebsiteStatus {
            url,
            action_status: Err(format!("Failed to create HTTP client: {}", e)),
            response_time: Duration::from_secs(0),
            timestamp: SystemTime::now(),
        },
    };

    let start_time = Instant::now();
    let mut last_result: Result<u16, String> = Err("Initial check not attempted".to_string());

    for attempt in 0..=retries {
        last_result = match client.get(&url).send() {
            Ok(response) => Ok(response.status().as_u16()),
            Err(e) => Err(format!("Request error: {}", e)),
        };

        if last_result.is_ok() || attempt == retries {
            break;
        }

        thread::sleep(Duration::from_millis(100));
    }

    let response_time = start_time.elapsed();
    let timestamp = SystemTime::now();

    WebsiteStatus {
        url,
        action_status: last_result,
        response_time,
        timestamp,
    }
}

fn main() -> Result<(), String> {
    let config = parse_arguments()?;

    let mut all_urls = config.urls;
    if let Some(file_path) = &config.file_path {
        match read_urls_from_file(file_path) {
            Ok(urls_from_file) => all_urls.extend(urls_from_file),
            Err(e) => eprintln!("Warning: Could not read URLs from file '{}': {}", file_path.display(), e),
        }
    }

    if all_urls.is_empty() {
        eprintln!("No URLs to check.");
        return Ok(());
    }

    let num_workers = config.workers;
    let timeout = Duration::from_secs(config.timeout);
    let retries = config.retries;

    // channels to communicate between threads
    let (url_tx, url_rx) = channel::<String>();
    let url_rx = Arc::new(Mutex::new(url_rx));
    let results = Arc::new(Mutex::new(Vec::new()));

    //worker threads
    let mut handles = Vec::new();
    for _ in 0..num_workers {
        let rx_clone = Arc::clone(&url_rx);
        let results_clone = Arc::clone(&results);
        let timeout_clone = timeout;
        let retries_clone = retries;

        let handle = thread::spawn(move || {
            loop {
                // get next url
                let url = {
                    let rx = rx_clone.lock().unwrap();
                    match rx.recv() {
                        Ok(url) => url,
                        //no more urls
                        Err(_) => break,
                    }
                };

                let status = check_website(url.clone(), timeout_clone, retries_clone);

                // output result
                println!(
                    "{} - Status: {}, Response Time: {:?}, Timestamp: {:?}",
                    status.url,
                    match &status.action_status {
                        Ok(code) => format!("{}", code),
                        Err(err) => err.clone(),
                    },
                    status.response_time,
                    status.timestamp
                );

                //store reusult
                let mut res = results_clone.lock().unwrap();
                res.push(status);
            }
        });
        handles.push(handle);
    }

    for url in all_urls {
        if let Err(e) = url_tx.send(url) {
            eprintln!("Warning: Failed to send URL to worker thread: {}", e);
        }
    }
    drop(url_tx); // close channel

    // worker threads ar finishing
    for handle in handles {
        if let Err(_e) = handle.join() {
            eprintln!("Warning: A worker thread panicked");
        }
    }

    // json file
    let final_results = results.lock().unwrap();
    let json_array: Vec<_> = final_results.iter().map(|status| {
        let status_code = match &status.action_status {
            Ok(code) => json!(code),
            Err(_) => json!(null),
        };
        let error_message = match &status.action_status {
            Ok(_) => json!(null),
            Err(err) => json!(err),
        };

        json!({
            "url": status.url,
            "status_code": status_code,
            "response_time_ms": status.response_time.as_millis(),
            "timestamp": format!("{:?}", status.timestamp),
            "error": error_message
        })
    }).collect();

    let json_string = serde_json::to_string_pretty(&json_array)
        .map_err(|e| format!("Error serializing to JSON: {}", e))?;

    let mut file = File::create("status.json")
        .map_err(|e| format!("Error creating status.json: {}", e))?;
    file.write_all(json_string.as_bytes())
        .map_err(|e| format!("Error writing to status.json: {}", e))?;

    println!("Results saved to status.json");

    Ok(())
}