package newfamilies.classes

import kotlin.reflect.KClass
import newfamilies.dependency.ClassMark
import newfamilies.dependency.relay
import newfamilies.dependency.head
import newfamilies.dependency.identity
import newfamilies.dependency.UnsafeGenerics
import java.util.ArrayList

class FixtureType(val number: Int)
var calls: Int = 0
val String: Any = 7

fun booleanType(): KClass<Boolean> = Boolean::class
fun byteType(): KClass<Byte> = Byte::class
fun shortType(): KClass<Short> = Short::class
fun charType(): KClass<Char> = Char::class
fun intType(): KClass<Int> = Int::class
fun longType(): KClass<Long> = Long::class
fun floatType(): KClass<Float> = Float::class
fun doubleType(): KClass<Double> = Double::class
fun stringType(): KClass<String> = kotlin.String::class
fun anyType(): KClass<Any> = Any::class
fun sourceType(): KClass<FixtureType> = FixtureType::class
fun importedType(): KClass<StringBuilder> = StringBuilder::class
fun genericType(): KClass<ArrayList<*>> = ArrayList::class
fun listType(): KClass<List<*>> = List::class
fun intArrayType(): KClass<IntArray> = IntArray::class
fun byteArrayType(): KClass<ByteArray> = ByteArray::class
fun charArrayType(): KClass<CharArray> = CharArray::class
fun stringArrayType(): KClass<Array<String>> = Array<String>::class
fun boxedArrayType(): KClass<Array<Int>> = Array<Int>::class
fun nestedArrayType(): KClass<Array<IntArray>> = Array<IntArray>::class
fun nullableElementArrayType(): KClass<Array<String?>> = Array<String?>::class
fun valueShadow(): KClass<out Any> = String::class
fun localShadow(): KClass<out Any> { val Int: Any = "local"; return Int::class }
fun boundAny(value: Any): KClass<out Any> = value::class
fun boundInterface(value: CharSequence): KClass<out CharSequence> = value::class
fun boundInt(value: Int): KClass<out Int> = value::class
fun boundString(value: String): KClass<out String> = value::class
fun boundArray(value: Array<String>): KClass<out Array<String>> = value::class
fun asserted(value: String?): KClass<out String> = value!!::class
fun nextInt(): Int { calls += 1; return 7 }
fun nextText(): CharSequence { calls += 1; return "effects" }
fun failValue(): Any { calls += 1; throw IllegalStateException("class-effect") }
fun intEffect(): KClass<out Int> = nextInt()::class
fun textEffect(): KClass<out CharSequence> = nextText()::class
fun failEffect(): KClass<out Any> = failValue()::class
fun javaInt(): Class<Int> = Int::class.java
fun javaString(): Class<String> = kotlin.String::class.java
fun javaArray(): Class<Array<String>> = Array<String>::class.java
fun javaBound(value: Any): Class<out Any> = value::class.java
fun javaBoundInt(value: Int): Class<out Int> = value::class.java
fun javaBoxedInt(): Class<Int> = Int::class.javaObjectType
fun javaPrimitiveInt(): Class<Int>? = Int::class.javaPrimitiveType
fun javaPrimitiveString(): Class<String>? = kotlin.String::class.javaPrimitiveType
fun kotlinClass(value: Class<String>): KClass<String> = value.kotlin
fun relayType(): Class<Int> = Int::class.relay
fun relayBound(value: Any): Class<out Any> = value::class.relay
fun simpleName(): String = FixtureType::class.java.simpleName
fun className(value: Any): String = value::class.java.name
fun threadName(value: Thread): String = value.name

class Outer { class Nested }
fun nestedType() = Outer.Nested::class
fun <T> identitySource(value: T): T = value
fun erasedSource(): Class<out Int> = identitySource(42)::class.java
fun erasedLibrary(): Class<out Int> = identity(42)::class.java
fun erasedProperty(values: List<Int>): Class<out Int> = values.head::class.java
fun erasedInvocation(block: () -> Int): Class<out Int> = block()::class.java
fun assertedInt(value: Int?): Class<out Int> = value!!::class.java
fun castInt(value: Any): Class<out Int> = (value as Int)::class.java
fun listElement(values: List<Int>): Class<out Int> = values[0]::class.java
fun boxedArrayElement(values: Array<Int>): Class<out Int> = values[0]::class.java
fun primitiveArrayElement(values: IntArray): Class<out Int> = values[0]::class.java
fun conditional(value: Boolean): Class<out Int> = (if (value) identitySource(42) else 2)::class.java

fun erasedUnchecked(): Class<out Int> = UnsafeGenerics.value<Int>()::class.java
fun storedPrimitive(): Class<out Int> { val value = identitySource(42); return value::class.java }
fun initializationType(): KClass<newfamilies.dependency.InitializationProbe> = newfamilies.dependency.InitializationProbe::class
fun initializationJava(): Class<newfamilies.dependency.InitializationProbe> = newfamilies.dependency.InitializationProbe::class.java
fun initializationBound(): KClass<out newfamilies.dependency.InitializationProbe> = newfamilies.dependency.InitializationProbe.create()::class
fun publicLibraryType(): KClass<newfamilies.dependency.PublicKotlinType> = newfamilies.dependency.PublicKotlinType::class
fun publicLibraryNested(): KClass<*> = newfamilies.dependency.PublicOuter.PublicNested::class

fun parenthesizedArray(): KClass<Array<String>> = (Array<String>)::class

fun <T: Any> genericBound(value: T): KClass<out T> = value::class
fun <U: Any, T: U> genericBoundChain(value: T): KClass<out T> = value::class

fun qualifiedArray(): KClass<Array<String>> = kotlin.Array<kotlin.String>::class
fun escapedBound(`value$slot`: Any): KClass<out Any> = `value$slot`::class

@ClassMark(kotlin.String::class)
fun classMarked() {}
@ClassMark(Short::class)
fun shortClassMarked() {}
fun shortBoxedArrayType(): KClass<Array<Short>> = Array<Short>::class
