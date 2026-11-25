package anonymousgeneric
interface Mapper<T> { fun map(value: T): T }
open class Base<T>(val seed: T) { open fun apply(value: T): T = value }
open class Middle<U>(value: U) : Base<U>(value)
