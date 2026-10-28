class Cell<T>(initial: T) { var value: T = initial
 private set
fun setInt(other: Cell<Int>, value: Int) { other.value = value } }
