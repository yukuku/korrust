package staticprobe.shadow
@Target(AnnotationTarget.FIELD)
@Retention(AnnotationRetention.BINARY)
annotation class JvmField
object Shadowed { @JvmField val value: Int = 3 }
