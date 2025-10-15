@garrett I found out with ```GDK_DEBUG=misc``` that I was running on the x11 backend.

This morning I did some tests with only one screen and I didn't experience any freezing, but I can't stay with only one monitor. 

I try to run on Wayland with ```GDK_BACKEND=wayland cargo run``` and I didn't experience any freezing up to now. 


I did the same test with x11 ```GDK_BACKEND=x11 cargo run``` and I sadly experience freezings.


Sorry for my naivety I was thinking that I was running on Wayland since Gnome shows the Wayland setting. I never set ```GDK_BACKEND``` before. I will look the documentation to see the pro and cons of the ```GDK_BACKEND``` settings.


I stay available for any questions

Thank you
