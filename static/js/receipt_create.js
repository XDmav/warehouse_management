let goods = [];

async function loadGoods() {
    const res = await fetch("/api/goods");

    goods = await res.json();
}

function addItem() {
    let row = document.createElement("tr");

    row.innerHTML = `
        <td>
            <input
                type="text"
                placeholder="Поиск..."
                oninput="filterGoods(this)"
                class="border p-1 mb-1">
            
            <select
                name="goods_id"
                class="border p-1 goods_select"
                onchange="goodsChanged(this)">
            
                ${goods.map(g => `<option value="${g.id}" data-price="${g.price}">${g.name}</option>`).join("")}
            </select>
        </td>
        
        <td>
            <span class="stock">—</span>
        </td>
        
        <td>
            <input type="number" name="quantity" value="1" min="1"
            oninput="updateRow(this)" class="border p-1 w-20">
        </td>
        
        <td>
            <input type="number" name="price"
            step="0.01"
            oninput="updateRow(this)"
            class="border p-1 w-24">
        </td>
        
        <td>
            <input type="number" name="discount"
            value="0"
            step="0.01"
            oninput="updateRow(this)"
            class="border p-1 w-20">
        </td>
        
        <td class="row_total">0</td>
        
        <td>
            <button type="button" onclick="removeRow(this)">✕</button>
        </td>
    `;

    document.getElementById("items").appendChild(row);

    goodsChanged(row.querySelector("select"));
}

async function goodsChanged(select) {
    let option = select.selectedOptions[0];

    let price = option.dataset.price;

    let row = select.closest("tr");

    row.querySelector("[name=price]").value = price;

    let goodsId = select.value;

    let res = await fetch(`/api/goods/${goodsId}/stock`);

    let data = await res.json();

    let stock = data.stock;

    let stockCell = row.querySelector(".stock");

    stockCell.innerText = stock;

    if (stock <= 0) {
        row.style.backgroundColor = "#ffcccc";
    } else {
        row.style.backgroundColor = "";
    }

    updateRow(select);
}


function removeRow(btn) {
    btn.closest("tr").remove();

    updateTotal();
}


function updateRow(el) {
    let row = el.closest("tr");

    let qty = parseFloat(row.querySelector("[name=quantity]").value) || 0;

    let price = parseFloat(row.querySelector("[name=price]").value) || 0;

    let discount = parseFloat(row.querySelector("[name=discount]").value) || 0;

    let sum = qty * price * (1 - discount / 100);

    row.querySelector(".row_total").innerText = sum.toFixed(2);

    updateTotal();
}


function updateTotal() {
    let total = 0;

    document.querySelectorAll(".row_total").forEach(e => {
        total += parseFloat(e.innerText) || 0;
    });

    document.getElementById("total").innerText = total.toFixed(2);
}


function filterGoods(input) {
    let filter = input.value.toLowerCase();

    let select = input.nextElementSibling;

    for (let o of select.options) {
        o.style.display = o.text.toLowerCase().includes(filter) ? "" : "none";
    }
}


document.addEventListener("DOMContentLoaded", async () => {
    await loadGoods();

    document.getElementById("receipt_date").value =
        new Date().toISOString().slice(0, 10);

    addItem();
});