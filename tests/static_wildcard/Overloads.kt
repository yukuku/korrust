package star.consumer
import star.library.First.*
import star.library.Second.*
fun fromFirst(): Int = choose(5)
fun fromSecond(): Int = choose(5L)
fun separate(): Int = left(1) + right(2)
