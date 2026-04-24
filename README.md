# wlr-lite-widget

This library is designed to do very CPU and RAM efficient widgets for wayland  
Only compositors based on wlroots or smithay are supported

There is examples in `examples` folder that you can run with :
> cargo run --example front_clock

This examples gives a binary of 2 Mo that use less than 4 Mo of RAM while running

Most important part of this library is that drawing are done only when required (e.g. once per second for the clock example)
and can be done on a specific part of the widget (no need to redraw the entire widget each time)

You can use the backend that you want, CPU backend are recommended since the final image is sent by CPU, I have tested with cairo and tiny-skia  
I plan to add easier implementation for tiny-skia and cairo (as feature of the library)

## build

I am working on a Pull Request on wayland-client (and on SCTK after release of wayland-client with my modifications)  
But currently, you need to edit these libs in order to compile this, everything is exlained at the end of widget.rs  
I know that this is not a good practice, if you need help with that, feel free to ask