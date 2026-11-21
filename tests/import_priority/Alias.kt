package priority.consumer
import priority.two.*
import priority.one.StringBuilder as Chosen
import kotlin.jvm.JvmField as Exposed
class Aliased(@Exposed val value: String, val builder: Chosen)
