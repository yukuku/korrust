import aliasdep.Bounded
typealias Constrained<T> = Bounded<T>
fun bad(value: Constrained<Int>): Int = 1
