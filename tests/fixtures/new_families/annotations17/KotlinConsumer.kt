package annotation17consumer

import annotation17.*
import kotlin.reflect.full.primaryConstructor
import kotlin.reflect.full.declaredMemberProperties

@Rich(number = 51L, nested = Nested("consumer", 53L, Mode.FIRST), children = [Nested("child", 59L, Mode.SECOND)], types = [Token::class, DoubleArray::class])
class ConsumerExplicit

@Rich
class ConsumerDefaults

@Pack(Nested("consumer pack", 61L, Mode.SECOND))
class ConsumerPack

fun main() {
    val explicit = ConsumerExplicit::class.java.getAnnotation(Rich::class.java)
    check(explicit.number == 51L)
    check(explicit.nested.text == "consumer")
    check(explicit.nested.amount == 53L)
    check(explicit.nested.mode == Mode.FIRST)
    check(explicit.children[0].amount == 59L)
    check(explicit.types[0] == Token::class)
    check(explicit.types[1] == DoubleArray::class)
    val defaults = ConsumerDefaults::class.java.getAnnotation(Rich::class.java)
    check(defaults.number == Long.MIN_VALUE)
    check(defaults.letter == '\uD800')
    check(defaults.names[2] == "\uD800")
    check(defaults.nested.amount == 17L)
    check(ConsumerPack::class.java.getAnnotation(Pack::class.java).entries[0].amount == 61L)
    val constructor = Rich::class.primaryConstructor!!
    check(constructor.parameters.size == 16)
    check(constructor.parameters.all { it.isOptional })
    check(constructor.parameters.map { it.name } == listOf("number", "ratio", "measure", "letter", "mode", "type", "numbers", "flags", "letters", "ratios", "measures", "names", "modes", "types", "nested", "children"))
    check(Rich::class.declaredMemberProperties.map { it.name }.toSet() == constructor.parameters.map { it.name }.toSet())
    val pack = Pack::class.primaryConstructor!!.parameters.single()
    check(pack.name == "entries")
    check(pack.isVararg)
    check(!pack.isOptional)
    check(Rich::class.annotations.filterIsInstance<Meta>().single().id == "rich")
    println("OK:21")
}
