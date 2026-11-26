package composeprobe.lookalike
@Target(AnnotationTarget.TYPE)
annotation class Composable
fun accept(block: @Composable () -> Unit) { block() }
