package composetypeprobe
import androidx.compose.runtime.Composable
fun unsupported(): @Composable () -> Unit = { val nested: @Composable () -> Unit = {}; nested() }
