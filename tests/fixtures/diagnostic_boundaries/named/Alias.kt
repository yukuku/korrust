package named
import named.Visible as Alias
fun aliased(): String = Alias::class.java.simpleName
