package priority.one
@Target(AnnotationTarget.FIELD)
@Retention(AnnotationRetention.BINARY)
annotation class JvmField
class StringBuilder(val token: Int)
class Marker(val code: Int)
typealias AliasBuilder = StringBuilder
