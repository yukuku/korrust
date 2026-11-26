package packagevalue
import fields.Base
val CODE = 67
class ValueChild : Base() { fun code(): Int = CODE }
class Scoped : Base() { companion object { val CODE: Int = 71 }; fun code(): Int = CODE }
class Exposed : Base() { companion object { @JvmField val CODE: Int = 73 }; fun code(): Int = CODE }
