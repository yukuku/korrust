package priority.consumer
import priority.one.*
import priority.two.StringBuilder
import kotlin.jvm.JvmField
class Explicit(@JvmField val value: String, val builder: StringBuilder)
