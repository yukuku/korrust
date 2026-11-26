package fixtures
fun objects(n: Int): Array<Element?> = arrayOfNulls<Element>(size = n)
fun boxed(n: Int): Array<Int?> = arrayOfNulls(n)
fun nested(): Array<Array<String>?> = arrayOfNulls<Array<String>>(2)
