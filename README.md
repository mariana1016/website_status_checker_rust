# Website Checker

This Rust project application checks the HTTP status of websites. It also provides the  live output and saves the results in a JSON file.

## Build Instructions

In order to run the application, use this command:

```bash
cargo build --release
```
I chose to use this command since it is to easy to execute when accessing the target/release/website_checker directory.
-

This is the format of the command 
```bash
./target/release/website_checker [--file <sites.txt>] [suspicious link removed] [--workers N] [--timeout S] [--retries N]
```
## This is what each [ ] does

--file <sites.txt>: This specifies a text file containing a list of URLs to check, but it works only one URL per line. This references the example of 50 sites that I provided (sites.txt). You can use any valid website, but I provided examples since it was a requirement \
suspicious link removed: You can use one or more URLs as command line arguments \
--workers N: the number of worker threads we are going to use \
--timeout S: the timeout for each HTTP request \
--retries N: the amount of retries after each URL failure  \

## One URL at a time
This command allows you to check a single URL 
```bash
./target/release/website_checker [http://www.google.com](http://www.google.com)
```

## Multiple URLs
This command allows you to check multiple URLs at the same time
```bash
./target/release/website_checker [http://www.google.com](http://www.google.com) [http://www.example.com](http://www.example.com) [http://www.rust-lang.org](http://www.rust-lang.org)
```
## Get a URL from a file 
This command allows you to get a URL from the sites.txt file I provided
```bash
./target/release/website_checker --file sites.txt
```
## You can also use a mix of both 
```bash
./target/release/website_checker --file sites.txt [http://www.twitter.com](http://www.twitter.com)
```
## Number of workers, timeouts, and retries
```bash
./target/release/website_checker --file sites.txt --workers 4 --timeout 10 --retries 3
```
## Output
At first, the program is going to print a line for each URL checked. This printed line shows the URL, HTTP status code, and the response time. Then, after the URLs have been checked, the results are saved into a JSON file called status.json. The file contains a JSON array of objects; each object represents the status of each website.

## status.json
The array of objects is these: \
URL: this is just the URL \
status code: HTTP status code, if there's an error, it's null \
success: this tells us if it worked or not (successful or failure) \
error: if the URL wasn't successful, then it's null. 

## Error Handling 
If no files are provided when trying to run, the program will print a message on how the command is supposed to look. \
If there are issues with the serialization of JSON, it will also make the program exit with an error message \
### Warnings 
Warnings can appear when trying to run the code if: \
The program couldn't read the URLs from the file provided \
The URL wasn't sent to the worker thread \
A worker thread panics
