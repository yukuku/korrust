package newfamilies.aliases

import aliasdep.Box
import aliasdep.Bounded

typealias Rows<T> = List<T>
typealias MaybeRows<T> = Rows<T>?
typealias Transformer<T> = (T?) -> T
typealias Converter<A, B> = (A) -> B
typealias Format<T> = T.(Int) -> String
typealias Slots<T> = Array<T?>
typealias Nested<T> = List<List<T?>>
typealias Ignored<T> = String
typealias IgnoredChain<T> = Ignored<T>
typealias OptionalTransform<T> = ((T) -> T)?
typealias Boxed<T> = Box<T>
typealias Constrained<T> = Bounded<T>
typealias TripleRows<T> = List<List<List<T>>>
typealias MutableRows<T> = List<MutableList<T>>
typealias MutableOuterRows<T> = MutableList<List<T>>
typealias NullableRows<T> = List<List<T>?>
typealias FunctionRows<T> = List<(T) -> String>
typealias BoxRows<T> = List<Box<T>>
typealias ArrayRows<T> = List<Array<T>>
private typealias PrivateRows<T> = List<T>

fun privateAliasSize(value: PrivateRows<Int>): Int = value.size
