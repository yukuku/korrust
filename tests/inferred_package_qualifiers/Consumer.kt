package sample
class Example {
    val numbers = listOf(1) + (0 until Values.LIMIT).map { Values.twice(it) }
}
val global = listOf(1) + (0 until Values.LIMIT).map { Values.twice(it) }
