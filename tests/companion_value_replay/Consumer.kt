package companionvalues.consumer
import companionvalues.library.Key
import companionvalues.library.Palette
fun primary(): Long = Palette.primary.raw + Key.zero.raw
fun name(): String = Palette.name.raw + ":" + Palette.javaName
fun mutate(): Long {
    Palette.accent = Key(67L)
    return Palette.accent.raw
}
fun nullable(): Boolean = Palette.maybe == null
fun version(): Int = Palette.version
fun defaultMethod(): Long = Palette.choose().raw
