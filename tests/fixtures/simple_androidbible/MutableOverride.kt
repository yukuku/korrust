package simple.mutable

import simple.mutable.ReadOnlyBase

interface MutableOverride : ReadOnlyBase {
    override var value: Int
}
