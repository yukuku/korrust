package simple.mutable

import simple.mutable.MutableSurface

interface InvalidOverride : MutableSurface {
    override val count: Int
}
