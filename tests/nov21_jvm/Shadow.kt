package novjvm.shadow

@Target(AnnotationTarget.FUNCTION, AnnotationTarget.CONSTRUCTOR)
@Retention(AnnotationRetention.RUNTIME)
annotation class Throws(val value: String)
@Target(AnnotationTarget.FUNCTION)
@Retention(AnnotationRetention.RUNTIME)
annotation class Synchronized

@Throws("ordinary") @Synchronized
fun plain(): Int = 16

object Holder {
    @Throws("object") @Synchronized
    fun plain(): Int = 17
}
