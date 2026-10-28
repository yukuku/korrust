package newfamilies.consumer

import kotlin.reflect.KClass
import newfamilies.classes.*

fun check(): String {
    val nested: KClass<Outer.Nested> = nestedType()
    if (nested.java != Outer.Nested::class.java) return "nested"
    val boundedClass: KClass<out String> = genericBound("bounded")
    val genericChain: KClass<out String> = genericBoundChain("chain")
    if (boundedClass != kotlin.String::class || genericChain != kotlin.String::class) return "generic-bound"
    if (escapedBound(`value$slot` = "named") != kotlin.String::class) return "escaped"
    val integer: KClass<Int> = intType()
    val strings: KClass<Array<String>> = stringArrayType()
    val generic: KClass<List<*>> = listType()
    val source: KClass<FixtureType> = sourceType()
    val bound: KClass<out Any> = boundAny(5)
    val reflected: Class<out Any> = javaBound(5)
    val relayed: Class<out Any> = relayBound(5)
    if (integer.java != Int::class.java) return "integer"
    if (strings.java != Array<String>::class.java) return "array"
    if (generic.java != List::class.java) return "generic"
    if (source.java != FixtureType::class.java) return "source"
    if (bound != Int::class) return "bound"
    if (reflected != java.lang.Integer::class.java) return "runtime"
    if (relayed != reflected) return "relay"
    if (source.java.getConstructor(Int::class.java).newInstance(7).number != 7) return "metadata"
    return "OK"
}
