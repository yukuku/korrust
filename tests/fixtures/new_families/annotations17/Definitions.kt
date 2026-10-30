package annotation17

import kotlin.reflect.KClass

enum class Mode { FIRST, SECOND }
class Token

@Retention(AnnotationRetention.RUNTIME)
@Target(AnnotationTarget.ANNOTATION_CLASS)
annotation class Meta(val id: String)

@Meta("nested")
@Retention(AnnotationRetention.RUNTIME)
@Target(AnnotationTarget.ANNOTATION_CLASS)
annotation class Nested(val text: String = "nested", val amount: Long = 17L, val mode: Mode = Mode.SECOND)

@Meta("rich")
@Retention(AnnotationRetention.RUNTIME)
@Target(AnnotationTarget.CLASS, AnnotationTarget.FUNCTION, AnnotationTarget.PROPERTY, AnnotationTarget.FIELD, AnnotationTarget.VALUE_PARAMETER, AnnotationTarget.PROPERTY_GETTER, AnnotationTarget.PROPERTY_SETTER, AnnotationTarget.ANNOTATION_CLASS)
annotation class Rich(
    val number: Long = -9223372036854775807L - 1L,
    val ratio: Float = -0.0f,
    val measure: Double = 1.25,
    val letter: Char = '\uD800',
    val mode: Mode = Mode.SECOND,
    val type: KClass<*> = Token::class,
    val numbers: LongArray = longArrayOf(0L, 1L, -1L),
    val flags: BooleanArray = booleanArrayOf(true, false),
    val letters: CharArray = charArrayOf('\u0000', '\uD800', '\uDFFF'),
    val ratios: FloatArray = floatArrayOf(-0.0f, 1.25f),
    val measures: DoubleArray = doubleArrayOf(-0.0, 2.5),
    val names: Array<String> = arrayOf("", "\u0000", "\uD800", "\uD83D\uDE00"),
    val modes: Array<Mode> = arrayOf(Mode.FIRST, Mode.SECOND),
    val types: Array<KClass<*>> = arrayOf(Int::class, Array<String>::class, IntArray::class, Unit::class, Token::class),
    val nested: Nested = Nested(),
    val children: Array<Nested> = arrayOf(Nested("first", 1L, Mode.FIRST), Nested()),
)

@Retention(AnnotationRetention.BINARY)
@Target(AnnotationTarget.CLASS)
annotation class BinaryMark(val mode: Mode = Mode.FIRST)

@Retention(AnnotationRetention.SOURCE)
@Target(AnnotationTarget.CLASS)
annotation class SourceMark(val text: String)

@Retention(AnnotationRetention.RUNTIME)
@Target(AnnotationTarget.CLASS)
annotation class Pack(vararg val entries: Nested)

@Retention(AnnotationRetention.RUNTIME)
@Target(AnnotationTarget.CLASS)
annotation class ClassOnly

