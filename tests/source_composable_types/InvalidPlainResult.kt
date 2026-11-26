package composetypeprobe
import androidx.compose.runtime.Composable
fun invalid(callback: () -> Unit): @Composable () -> Unit = callback
