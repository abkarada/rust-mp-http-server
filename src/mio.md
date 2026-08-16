
İlk adım TCPStream soketlerini Non-Blocking modta çalıştırmak:
Neden: CPU run queue'dan process in çıkarılmaması ve de anında return edilip bloklanmaması için.

Eğer I/O blocking değilde Non-blocking modta çalışırsa veri 
gelmediğinde hata verir bunu engellemek için:
Busy-waiting yapılamaz cpu yu yakarsın
Soketi bir olaya kaydedersin? Ne demek:
OS seviyesinde şunu dersin:
Ben bu soketi şu olaylar olmadıkça uyutuyorum ama eğer kaydettirdiğim
olaylar olursa onu uyandır:Busy wait değil, event-driven denmesinin
sebebi bu olay olunca harekete geçiyo.

mio lib'te bunu yapan yapı 'Poll'

Resmi Dökümentasyonda :
"Using Mio starts by creating a Poll, which reads events from the OS and puts them into Events. You can handle I/O events from the OS with it."
yazıyor.

1) Poll oluştur:

let mut poll = Poll::new()?; // Poll oluşturuldu

2) Events struct ı oluştur, pollenen olaylar buraya gönderilecek ve bunun kapasitesini belirle:

let events = Events::with_capacity(1024); /* diyelim ki OS 10.000 tane socket dinliyo ve
birden 3.000 tane soketin SO_RECV buffer'ına veri geldi bu durumda hepsini işleyemezsin 
hem RAM hem CPU mâliyeti çok yüksek hemde gelen olayların ne kadar süreceğini bilmiyosun
bu yüzden user space e iletilecek olay sayısını belirlemen gerekir çok küçük seçersen 
cpu kernel space ten user space e bu olayları getirmek için sürekli context-switch yapar
eğer bu sayı çok büyük olursa döngünün başında çok büyük bir bellek alanının sıfırlanması
ve işlenmesi CPU'nun L1/L2 önbelleklerine(Cache) sığmaz ve cache miss yaşanarak performans sorunlarına neden olur.

En uygun sayı 1024 - 4096 arasında ölçülmüştür(resmi dökümentasyona göre) */

3) Benzersiz bir token oluştur ki OS'un gözlemlediği fd'den bişey gelirse bu token dönsün ve bizde anlayabilelim.

const SERVER : Token = Token(0); //Genelde ya 0 ya da 1024 tercih edilir

4) Gözlenecek bir fd yarat: Ne gözlemeyeceğine bağlı, genelde bir I/O gözlemlenir:

let mut server : TcpListener = TcpListener::bind("buraya string olarak bir adres gir")?;

5) Bu I/O fd'sini poll'a kaydet:

poll.registry.register(&mut server, SERVER, Interest:READABLE)?; /*
Buradaki interest tam olarak fd nin hangi durumlarda pollanacağını belirtir mesela
server'da genelde bağlantı gelince recv() syscall'da ki SO_RECV buffer'ı dolar bu durumda
tepki vermek isteriz yani bu olaya ilgi gösteririz(Interest), fakat server'dan client'a
cevap yazıldığında write() syscall'ındaki SO_SEND buffer'ı dolar bu durumu kontrol etmemize gerek yoktur ilgi göstermeyiz.

6) Soketleri ile uğraşmak  için reactor pattern + worker pool mimarisi kurmak en uygunu olur. İlk başta bunu reactor pattern ile tek threadli şekilde yapacağım sonrasında 
worker pool'da ki senkronizasyonu sağlamak için mpsc gibi bir multi producer single consumer mantığı kuracağım ve bunlara bir stealer algoritması yazağım.


loop { // event loop başlangıcı
    poll.poll(&mut events, Some(Duration::from_millis(100)))?; /*
    Ağdan veri gelirse thread'i ANINDA uyandır (0 milisaniye gecikme). Ancak ağ tamamen sessizse bile, thread'i en fazla 100 milisaniye uyut; 100. milisaniyede mecburen uyandır(Bunu None, da yapabilirdik fakat birçok açıdan CPU'nun Run Queue'suna geri dönmeyi (uyanmayı) garanti altına almak isteriz)*/
    let next_client_id = 1;
    for event in &events { /* izlediğimiz bir I/O olayı tetiklendiğinde poll bunu doğrudan events struct ına gönderir bizde for ile içinde iterasyon yaparız */


    //gelen tcpstreamleri id ile eşleştirmek için hashmap oluştur:
    let client: HashMap<Token, TcpStream> = HashMap::new();

    match event.token() {
        SERVER => loop { /* SERVER(Token(0)) döndüğü anda cihaza bağlantı gelmiş demektir */

        // Bağlantıyı kabul edip poll'a kaydet:

        match server.accept(){
            Ok(client_stream, client_address) => {
                let client_token = Token(next_client_id);
                next_client_id += 1;

                poll.registery.register(&mut client_stream, client_token, Interest::READABLE)?;

                client.insert(client_token, client_stream);

            }

            Err(e) => if e.kind() == io::ErrorKind::WouldBlock => {
                                // Kabul edilecek başka bağlantı kalmadı, ana döngüye dön.
                                break;
                            }
                            Err(e) => return Err(e),
         }
   
            }

        }
    }
}

