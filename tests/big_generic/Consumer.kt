fun consume():String {
 val box=Box<String>("class")
 val captured:()->String=box.capture()
 val frozen:()->Int=box.freeze(7)
 val nested:()->()->String=box.nested("nested")
 val callback:()->String=nested()
 val mutable:()->String=box.mutable("cell")
 return captured() + frozen().toString() + callback() + mutable()
}
