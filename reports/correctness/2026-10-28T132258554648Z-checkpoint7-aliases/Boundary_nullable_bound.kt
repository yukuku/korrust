import aliasdep.Bounded
typealias Constrained<T> = Bounded<T>
fun bad(value: Constrained<String?>): Int = 1
