package sample
import library.Types
import library.Types.Base
import library.Types.Leaf
fun wider(x: Leaf?, y: Base) = x ?: y
fun narrower(x: Base?, y: Leaf) = x ?: y
fun items(x: Types?) = (x?.items ?: emptyList()).size
