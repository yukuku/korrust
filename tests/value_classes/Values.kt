package valuecases

import kotlin.jvm.JvmInline
import kotlin.jvm.JvmInline as ValueMarker

interface Show {
    fun show(): String
}

@JvmInline
value class Id(val raw: Int) {
    fun add(delta: Int): Id = Id(raw + delta)
    fun format(parts: List<String>): String = parts[0]
}

@JvmInline
value class Wide(val raw: Long) : Show {
    override fun show(): String = "Wide$raw"
}

@JvmInline
value class Text(val raw: String) {
    override fun toString(): String = raw
}

@JvmInline
value class MaybeText(val raw: String?)

@ValueMarker
value class Secret(private val raw: Int) {
    fun read(): Int = raw
}

object Effects {
    var count: Int = 0
}

@JvmInline
value class Tracked(val raw: Int) {
    init {
        Effects.count = Effects.count + 1
        if (raw < 0) throw IllegalArgumentException("negative")
    }
}

class Box<T>(val value: T)
class Holder(val value: Id) {
    fun capture(): () -> Id = { value }
}
data class Pair(val id: Id, val wide: Wide, val text: Text)

fun createId(raw: Int): Id = Id(raw)
fun createWide(raw: Long): Wide = Wide(raw)
fun createText(raw: String): Text = Text(raw)
fun createMaybeText(raw: String?): MaybeText = MaybeText(raw)
fun nullableId(value: Id?): Id? = value
fun nullableText(value: Text?): Text? = value
fun nullableMaybeText(value: MaybeText?): MaybeText? = value
fun asAny(value: Id): Any = value
fun fromAny(value: Any): Id = value as Id
fun generic(value: Id): Box<Id> = Box<Id>(value)
fun list(value: Id): List<Id> = listOf(value)
fun asShow(value: Wide): Show = value
fun fromShow(value: Show): Wide = value as Wide
fun tracked(raw: Int): Tracked = Tracked(raw)
fun nullMaybe(): MaybeText? = null
fun boxedNullMaybe(): MaybeText? = MaybeText(null)

fun capture(value: Id): () -> Id = { value }

class Mixed {
    fun keep(parts: List<String>, value: Id?): Id? = value
}

fun mixed(parts: List<String>, value: Id?): List<Id> = listOf(Id(parts.size))
