package custom
@Target(AnnotationTarget.CONSTRUCTOR)
@Retention(AnnotationRetention.RUNTIME)
annotation class JvmOverloads
class Lookalike @JvmOverloads constructor(val required: Int, val optional: Int = 7)
