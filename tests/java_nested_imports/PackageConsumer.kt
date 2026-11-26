package nested.library

import nested.library.Outer.PackageItem
import nested.library.Outer.ProtectedItem
import nested.library.PackageOuter.Nested as PackageOuterNested

fun samePackage(): Int = PackageItem.number() + ProtectedItem.number() + PackageOuterNested.number()
