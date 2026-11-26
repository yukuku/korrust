package packagefield
import fields.Base
const val CODE = 59
class PackageChild : Base() { fun code(): Int = CODE }
class CompanionChild : Base() { companion object { const val CODE = 61 }; fun code(): Int = CODE }
