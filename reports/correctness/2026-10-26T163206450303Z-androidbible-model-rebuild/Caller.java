import yuku.alkitab.model.SingleChapterVerses;
public class Caller {
 public static void main(String[] args) {
  SingleChapterVerses empty=SingleChapterVerses.Companion.getEMPTY();
  if(empty.getVerseCount()!=0) throw new AssertionError("count");
  try { empty.getVerse(0); throw new AssertionError("missing failure"); }
  catch (IllegalArgumentException e) { if(!e.getMessage().equals("@@getVerse requesting verse_0=0 but verseCount=0")) throw new AssertionError(e); }
  System.out.println("PASS");
 }
}
