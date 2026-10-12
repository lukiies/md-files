# Załącznik E — Cennik (fragment do przykładu)

## E2. Stawka godzinowa za pracę ponad zakres lub pulę godzin: £/h (per Poziom)

*Dotyczy Poziomów: T1–T5.*

> 🔀 ⟦TABELA · kolumny: poziom⟧

| Poziom | T1 | T2 | T3 | T4 | T5 |
|---|--:|--:|--:|--:|--:|
| **Stawka £/h** | £110 | £95 | £85 | £80 | £75 |

a. Praca ponad zakres lub pulę godzin danego Poziomu (nadwyżka, Załącznik A definicja 8) jest rozliczana według stawki godzinowej z powyższej tabeli. Zakres prac w cenie danego Poziomu określa Załącznik A⟦JEŻELI tier >= T3⟧, a wielkość puli godzin Załącznik F poz. F11⟦KONIEC⟧.

b. Opłata odpowiada wyłącznie liczbie przepracowanych godzin. Nie nalicza się premii, dopłaty za przekroczenie puli ani opłaty minimalnej za zgłoszenie.

> 🔀 ⟦JEŻELI tier == T4⟧

c. Klient, który przewiduje większe zapotrzebowanie na godziny, może z góry wykupić większą pulę godzin z rabatem ilościowym (poz. E3). Im większy wykupiony pakiet, tym niższa stawka za godzinę.
> 🔀 ⟦KONIEC⟧


> 🔀 ⟦JEŻELI hours⟧

d. Za pracę w rozszerzonych godzinach pracy wsparcia stawkę godzinową powiększa się o dopłatę procentową właściwą dla wykupionego okna godzin (poz. E4).
> 🔀 ⟦KONIEC⟧


> 🔀 ⟦JEŻELI tier == T4⟧

## E3. Rezerwacja godzin: pula gwarantowana T4 (poziomy rezerwacji L1–L4)

*Dotyczy Poziomu: T4.*

| Poziom rezerwacji | Zarezerwowane h/mc | Dopłata £/mc |
|---|:--:|--:|
| ⟦WIERSZ reservation == L1⟧**L1 · w cenie** | 2 h | £0 (w bazie T4) |
| ⟦WIERSZ reservation == L2⟧**L2 · Standard** | 8 h | £512 |
| ⟦WIERSZ reservation == L3⟧**L3 · Development** | 24 h | £1 344 |
| ⟦WIERSZ reservation == L4⟧**L4 · Intensive** | 35 h | £1 820 |

*Im większy wykupiony pakiet godzin, tym niższa efektywna stawka za godzinę (rabat ilościowy). Reguły puli: Zał. A.*
> 🔀 ⟦KONIEC⟧


> 🔀 ⟦JEŻELI demands⟧

## E5. Świadczenia dodatkowe na żądanie (osobne, addytywne)

*Dotyczy Poziomów: T1–T5.*

| Poz. | Świadczenie | Opłata |
|---|---|---|
| ⟦WIERSZ holidays · zaślepka: numer⟧**E5.1** | Pokrycie w dni ustawowo wolne w PL („święta") | **+20%** RAZEM kontraktu (SP+EF+Infra+API) |
| ⟦WIERSZ standby24 · zaślepka: numer⟧**E5.2** | Całodobowy dyżur 24h (standby 24h), do 2 zarezerwowanych okien/mc | **+20%** RAZEM kontraktu |
| ⟦WIERSZ standby24 · zaślepka: numer⟧**E5.3** | Dyżur 24h — 3. i kolejne okno w miesiącu | **£250 / okno** |

*Opłaty te są liczone na końcu, każda od kwoty podstawowej (nie są naliczane jedna od drugiej) i osobno od dopłat z poz. E4. Reguły: OWU §3 ust. 6.*
> 🔀 ⟦KONIEC⟧
