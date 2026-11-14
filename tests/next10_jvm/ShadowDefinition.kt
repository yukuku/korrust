package marks
@Target(AnnotationTarget.FILE)
@Retention(AnnotationRetention.BINARY)
annotation class JvmName(val name: String)
