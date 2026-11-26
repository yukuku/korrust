package composeprobe
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
interface MemoHost { fun hit(value: Int) }
@Stable interface StableMemoHost : MemoHost
interface InheritedStableMemoHost : StableMemoHost
object MemoEvents {
    var first: ((Int) -> Unit)? = null
    var second: (() -> Unit)? = null
    var third: (() -> Unit)? = null
    fun scalars(long: Long, double: Double, float: Float, flag: Boolean, char: Char, byte: Byte, short: Short) {
        Events.hit(long.toInt() + double.toInt() + float.toInt() + (if (flag) 1 else 0) + char.code + byte + short)
    }
}
@Composable fun callbacks(first: (Int) -> Unit, second: () -> Unit, third: () -> Unit) {
    MemoEvents.first = first
    MemoEvents.second = second
    MemoEvents.third = third
}
fun consumeInt(block: () -> Int) { block() }
fun consumePair(block: (Int, Int) -> Unit) { block(1, 2) }
