import transform.Words
import transform.transform
fun read(w: Words) = transform(4,w::pick)
fun length(w: Words) = transform("hello",w::pick)
fun format(value: Int): String = "number" + value
fun top() = transform(9,::format)
