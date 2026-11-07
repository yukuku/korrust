package filemarks

@Target(AnnotationTarget.FILE)
@Retention(AnnotationRetention.RUNTIME)
annotation class Visible(val text: String, val number: Int)

@Target(AnnotationTarget.FILE)
@Retention(AnnotationRetention.BINARY)
annotation class Binary(val flag: Boolean)

@Target(AnnotationTarget.FILE)
@Retention(AnnotationRetention.SOURCE)
annotation class Source(val text: String)

@Target(AnnotationTarget.CLASS)
annotation class ClassOnly
