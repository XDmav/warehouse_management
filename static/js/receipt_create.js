"use strict";

// Кэш списка товаров, загруженный с сервера
let goods = [];

async function loadGoods() {
    try {
        const res = await fetch("/api/goods", { credentials: "same-origin" });
        if (!res.ok) {
            showError(`Не удалось загрузить список товаров (код ${res.status})`);
            return;
        }
        goods = await res.json();
    } catch (e) {
        showError("Ошибка сети при загрузке товаров");
        console.error(e);
    }
}

async function fetchStock(goodsId) {
    try {
        const res = await fetch(
            `/api/goods/${encodeURIComponent(goodsId)}/stock`,
            { credentials: "same-origin" }
        );
        if (!res.ok) return null;
        const data = await res.json();
        return typeof data.stock === "number" ? data.stock : null;
    } catch (e) {
        console.error("fetchStock failed", e);
        return null;
    }
}

function filterOptions(input, selectId) {
    const select = document.getElementById(selectId);
    if (!select) return;
    filterSelect(input.value, select);
}

function filterGoods(input) {
    const select = input.parentElement.querySelector("select.goods_select");
    if (!select) return;
    filterSelect(input.value, select);
}

function filterSelect(query, select) {
    const needle = query.toLowerCase().trim();
    let firstVisible = null;

    for (const option of select.options) {
        const match = option.text.toLowerCase().includes(needle);
        option.hidden = !match;
        option.style.display = match ? "" : "none"; // fallback для старых браузеров
        if (match && !firstVisible) firstVisible = option;
    }

    if (firstVisible && select.selectedOptions[0]?.hidden) {
        select.value = firstVisible.value;
        // Если это select товаров — подтянуть цену и остаток для нового выбора
        if (select.classList.contains("goods_select")) {
            goodsChanged(select);
        }
    }
}

function addItem() {
    const row = document.createElement("tr");

    const tdGoods = document.createElement("td");

    const searchInput = document.createElement("input");
    searchInput.type = "text";
    searchInput.placeholder = "Поиск...";
    searchInput.className = "border p-1 mb-1";
    searchInput.addEventListener("input", () => filterGoods(searchInput));

    const select = document.createElement("select");
    select.name = "goods_id";
    select.className = "border p-1 goods_select";
    select.required = true;
    select.addEventListener("change", () => goodsChanged(select));

    for (const g of goods) {
        const opt = document.createElement("option");
        opt.value = g.id;
        opt.dataset.price = g.price;
        opt.textContent = g.name;
        select.appendChild(opt);
    }

    tdGoods.append(searchInput, select);

    const tdStock = document.createElement("td");
    const stockSpan = document.createElement("span");
    stockSpan.className = "stock";
    stockSpan.textContent = "—";
    tdStock.appendChild(stockSpan);

    const tdQty = document.createElement("td");
    const qtyInput = document.createElement("input");
    qtyInput.type = "number";
    qtyInput.name = "quantity";
    qtyInput.value = "1";
    qtyInput.min = "1";
    qtyInput.step = "1";
    qtyInput.required = true;
    qtyInput.className = "border p-1 w-20";
    qtyInput.addEventListener("input", () => updateRow(qtyInput));
    tdQty.appendChild(qtyInput);

    const tdPrice = document.createElement("td");
    const priceSpan = document.createElement("span");
    priceSpan.className = "price";
    priceSpan.textContent = "0.00";
    tdPrice.appendChild(priceSpan);

    const tdTotal = document.createElement("td");
    tdTotal.className = "row_total";
    tdTotal.dataset.value = "0";
    tdTotal.textContent = "0.00";

    const tdDel = document.createElement("td");
    const delBtn = document.createElement("button");
    delBtn.type = "button";
    delBtn.textContent = "✕";
    delBtn.className = "px-2";
    delBtn.addEventListener("click", () => removeRow(delBtn));
    tdDel.appendChild(delBtn);

    row.append(tdGoods, tdStock, tdQty, tdPrice, tdTotal, tdDel);
    document.getElementById("items").appendChild(row);

    goodsChanged(select);
}

async function goodsChanged(select) {
    const row = select.closest("tr");
    if (!row) return;

    const option = select.selectedOptions[0];
    if (!option) return;

    const priceSpan = row.querySelector(".price");
    const price = parseFloat(option.dataset.price) || 0;
    priceSpan.textContent = price.toFixed(2);

    const stockCell = row.querySelector(".stock");
    stockCell.textContent = "…";

    const stock = await fetchStock(select.value);
    if (stock === null) {
        stockCell.textContent = "?";
        row.style.backgroundColor = "";
    } else {
        stockCell.textContent = String(stock);
        row.style.backgroundColor = stock <= 0 ? "#ffcccc" : "";
    }

    updateRow(select);
}

function updateRow(el) {
    const row = el.closest("tr");
    if (!row) return;

    const qty = parseInt(row.querySelector("[name=quantity]").value, 10) || 0;
    const price = parseFloat(row.querySelector(".price").textContent) || 0;

    const sum = qty * price;

    const totalCell = row.querySelector(".row_total");
    totalCell.dataset.value = sum.toFixed(2);
    totalCell.textContent = sum.toFixed(2);

    updateTotal();
}

function removeRow(btn) {
    const row = btn.closest("tr");
    if (row) row.remove();
    updateTotal();
}

function updateTotal() {
    let total = 0;
    document.querySelectorAll(".row_total").forEach((cell) => {
        total += parseFloat(cell.dataset.value || cell.textContent) || 0;
    });
    const totalEl = document.getElementById("total");
    if (totalEl) totalEl.textContent = total.toFixed(2);
}

function showError(message) {
    const result = document.getElementById("result");
    if (!result) {
        alert(message);
        return;
    }
    result.textContent = message;
    result.className = "mt-4 text-red-500";
}

function todayLocalISO() {
    const d = new Date();
    const yyyy = d.getFullYear();
    const mm = String(d.getMonth() + 1).padStart(2, "0");
    const dd = String(d.getDate()).padStart(2, "0");
    return `${yyyy}-${mm}-${dd}`;
}

document.addEventListener("DOMContentLoaded", async () => {
    await loadGoods();

    const dateInput = document.getElementById("receipt_date");
    if (dateInput && !dateInput.value) {
        dateInput.value = todayLocalISO();
    }

    addItem();
});