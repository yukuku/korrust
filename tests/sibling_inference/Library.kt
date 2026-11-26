package fixture
fun <T> count(left: List<T>, right: List<T>): Int = left.size + right.size
fun <T> one(value: T): List<T> = listOf(value)
