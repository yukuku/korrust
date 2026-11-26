package consumers
import dep.Identity

class Fields(private var text: String?) {
    fun viaJava(): Any? = Identity.identity(text)
    fun read(): Any? = text
    fun assign(next: String?) { text = next }
}
