package simple.mutable

import simple.mutable.MutableSurface

interface InvalidOverride : MutableSurface {
    override var count: String
}
