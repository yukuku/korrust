package priority.consumer
import priority.one.*
class Wildcard(@JvmField val value: String, val builder: StringBuilder, val alias: AliasBuilder)
fun read(value: Wildcard): String = value.value
