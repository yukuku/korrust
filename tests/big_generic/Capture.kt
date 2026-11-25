class Box<C>(val seed:C) {
 fun capture():()->C = {seed}
 fun <T> freeze(value:T):()->T = { value }
 fun <T> transform(value:T): (T)->C = {seed}
 fun <T> mutable(value:T):()->T { var latest:T=value; return {latest=value; latest} }
 fun <T> nested(value:T):()->()->T = { val callback:()->T = {value}; callback }
}
