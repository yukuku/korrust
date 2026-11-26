package aliases.library

typealias Imported<T> = JavaTypes.Box<T>
typealias Chained<T> = Imported<T>
private typealias Hidden<T> = JavaTypes.Box<T>
class Closed<T> private constructor(val value: T)
typealias ClosedAlias<T> = Closed<T>
