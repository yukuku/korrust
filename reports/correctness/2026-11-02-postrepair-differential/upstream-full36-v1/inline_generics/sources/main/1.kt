
class Box<T>(val value: T) {
    inline fun run(block: (T) -> Unit) {
        block(value)
    }
}

