package composetypeprobe
import androidx.compose.runtime.Composable
fun unsupported(callback: @Composable () -> Unit, flag: Boolean): @Composable () -> Unit = { if (flag) callback() }
