package composetypeprobe
@Target(AnnotationTarget.TYPE) annotation class Other
fun unsupported(callback: @Other () -> Unit) {}
