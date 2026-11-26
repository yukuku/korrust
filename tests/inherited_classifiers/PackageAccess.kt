package inherited.library
import inherited.library.JavaTypes.Base
class PackageEnd: Base() {
    fun name(): String = PackageType.tag()
}
