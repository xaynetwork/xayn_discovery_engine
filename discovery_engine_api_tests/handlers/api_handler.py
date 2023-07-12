import json
import os

import pytest
import requests
import allure
import logging
from model.documents.documents import Documents

LOGGER = logging.getLogger(__name__)


class ApiHandler:
    TIMEOUT = 5

    def __init__(self):
        try:
            self.ingestion_endpoint = os.environ["INGESTION_URI"] + "/documents"
            self.personalization_endpoint = os.environ[
                                                "PERSONALIZATION_URI"] + "/users/{user_id}/personalized_documents"
            self.interactions_endpoint = os.environ["PERSONALIZATION_URI"] + "/users/{user_id}/interactions"
            self.semantic_search_endpoint = os.environ[
                                                "PERSONALIZATION_URI"] + "/semantic_search/{doc_id}"
        except KeyError:
            pytest.fail()

    def ingest_document(self, doc):
        docs = Documents(doc).to_json()
        return self.send_post_request(self.ingestion_endpoint, docs)

    def get_properties(self, doc_id):
        return self.send_get_request(self.ingestion_endpoint + "/" + doc_id + "/properties")

    def set_property(self, doc_id, properties):
        return self.send_put_request(self.ingestion_endpoint + "/" + doc_id + "/properties", data=properties)

    def interact_with_documents(self, user_id, interaction):
        """
        Method that takes user id and interaction object and sends patch to interaction endpoint
        :param user_id:
        :param data:
        :return:
        """
        return self.send_patch_request(self.interactions_endpoint.format(user_id=user_id), interaction)

    def delete_document(self, doc_id):
        return self.send_delete_request(self.ingestion_endpoint + "/" + doc_id)

    def get_personalized_docs(self, user_id, count=None, published=None):
        params = ''
        if count is not None:
            params += "?count=" + str(count)
        if published is not None:
            params += "?published_after=" + published
        return self.send_get_request(self.personalization_endpoint.format(user_id=user_id) + params)

    def get_semantic_search_doc(self, doc_id, count=None):
        if count is None:
            return self.send_get_request(self.semantic_search_endpoint.format(doc_id=doc_id))
        else:
            return self.send_get_request(self.semantic_search_endpoint.format(doc_id=doc_id) + "?count=" + str(count))

    # basic api calls used in other methods

    @allure.step
    def send_get_request(self, url):
        LOGGER.info("sending GET to " + url)
        return requests.get(url, timeout=self.TIMEOUT, headers={"Content-type": "application/json"})

    @allure.step
    def send_post_request(self, url, data):
        LOGGER.info("sending POST to " + url)
        return requests.post(url=url, data=data, timeout=self.TIMEOUT, headers={"Content-type": "application/json"})

    @allure.step
    def send_delete_request(self, url):
        LOGGER.info("sending DELETE to " + url)
        return requests.delete(url=url)

    @allure.step
    def send_patch_request(self, url, data):
        LOGGER.info("sending PATCH to " + url)
        return requests.patch(url=url, data=data, timeout=self.TIMEOUT, headers={"Content-type": "application/json"})

    @allure.step
    def send_put_request(self, url, data):
        LOGGER.info("sending PUT to " + url)
        return requests.put(url, data, timeout=self.TIMEOUT, headers={"Content-type": "application/json"})

    def deserialize_json(self, text):
        return json.loads(text)
