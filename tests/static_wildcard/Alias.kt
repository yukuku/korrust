package star.consumer
import star.library.First.*
import star.library.Second.*
import star.library.First.clash as alternate
fun alias(): Int = alternate(1) + clash(2)
