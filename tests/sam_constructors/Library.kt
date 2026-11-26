package samconstructors
fun interface Action {
    fun run(value: Int): Int
    fun twice(value: Int): Int = run(value) + run(value)
}
fun interface Mapper<T> { fun apply(value: T): T }
