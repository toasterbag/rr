# rr
A file "copyer" in the style of dd.

## Why?
I was trying to write a plan9 image to my SD card using dd and I never remember the correct flags
to get the progress of the transfer. It also failed to write the image for unknown reasons so instead of
searching the web for a fix I just Rewrote It In Rust!

Oh and did I mention it's async?

## How?
Install with
```bash
git clone https://github.com/toasterbag/rr.git
cd rr
cargo install --path ./
```
you can then run `rr --help` for instructions on how to use the program


## Warning
dd has been around for a long time for a reason so don't expect my version to offer the full
dd experience, it has not been rigourosly tested but it is good at writing plan9 images 
for my raspberry pi :)
