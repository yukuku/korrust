@file:OptIn("ordinary")
package fileshadow
@Target(AnnotationTarget.FILE)
annotation class OptIn(val text: String)
fun shadow(): Int = 17
